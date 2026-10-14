"""Tests for tools/backup_crypto.ps1 and the backup script's encryption (T-570).

The encrypted file must open with the user's decrypt-backups.ps1 (TLENC1), and
plaintext must never outlive a run, also a failed one. Needs pwsh 7.
"""

import os
import shutil
import subprocess
import tempfile
import unittest
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CRYPTO = ROOT / "tools" / "backup_crypto.ps1"
SCRIPT = ROOT / "tools" / "backup_supabase.ps1"
DECRYPT = ROOT / "tests" / "tlenc_decrypt.ps1"
PASSPHRASE = "correct horse battery"
PWSH = shutil.which("pwsh")


def pwsh(code, env_extra=None, temp=None):
    env = dict(os.environ)
    env.pop("TAVERN_BACKUP_PASSPHRASE", None)
    env.update(env_extra or {})
    if temp:
        env["TEMP"] = env["TMP"] = str(temp)
    return subprocess.run(
        [PWSH, "-NoProfile", "-NonInteractive", "-Command", code],
        capture_output=True, text=True, timeout=300, env=env, cwd=ROOT,
    )


@unittest.skipUnless(PWSH, "pwsh 7 not installed")
class BackupCryptoTest(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="tl-test-"))
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        self.temp = self.tmp / "temp"
        self.out = self.tmp / "out"
        self.temp.mkdir()

    def encrypt(self, work, passphrase=PASSPHRASE):
        target = self.out / "tl-backups-20260101-000000.tlenc"
        code = (
            f". '{CRYPTO}'; "
            f"Invoke-EncryptedBackup -OutFile '{target}' -FolderName 'run' "
            f"-Passphrase $env:TLENC_TEST_PASSPHRASE -Work {{ param($d) {work} }}"
        )
        res = pwsh(code, {"TLENC_TEST_PASSPHRASE": passphrase}, self.temp)
        return target, res

    def decrypt(self, target, passphrase=PASSPHRASE):
        zip_path = self.tmp / "plain.zip"
        res = pwsh(f"& '{DECRYPT}' -File '{target}' -Zip '{zip_path}'",
                   {"TLENC_TEST_PASSPHRASE": passphrase})
        return zip_path, res

    def test_round_trip_opens_with_the_tlenc1_decrypt_logic(self):
        work = (
            "Set-Content -NoNewline -Path (Join-Path $d 'data.sql') -Value 'synthetic rows'; "
            "New-Item -ItemType Directory (Join-Path $d 'storage') | Out-Null; "
            "Set-Content -NoNewline -Path (Join-Path $d 'storage/a.json') -Value 'json-content'"
        )
        target, res = self.encrypt(work)
        self.assertEqual(res.returncode, 0, res.stderr)
        blob = target.read_bytes()
        self.assertEqual(blob[:6], b"TLENC1")
        self.assertNotIn(b"synthetic rows", blob)

        zip_path, res = self.decrypt(target)
        self.assertEqual(res.returncode, 0, res.stderr)
        with zipfile.ZipFile(zip_path) as z:
            names = {n.replace("\\", "/") for n in z.namelist()}
            self.assertIn("run/data.sql", names)
            self.assertIn("run/storage/a.json", names)
            self.assertEqual(z.read("run/data.sql"), b"synthetic rows")

    def test_wrong_passphrase_does_not_decrypt(self):
        target, res = self.encrypt("Set-Content -Path (Join-Path $d 'x.txt') -Value 'x'")
        self.assertEqual(res.returncode, 0, res.stderr)
        _, res = self.decrypt(target, "another passphrase")
        self.assertNotEqual(res.returncode, 0)

    def test_two_runs_use_fresh_salt_and_nonce(self):
        a, _ = self.encrypt("Set-Content -Path (Join-Path $d 'x.txt') -Value 'x'")
        first = a.read_bytes()[6:34]
        b, _ = self.encrypt("Set-Content -Path (Join-Path $d 'x.txt') -Value 'x'")
        self.assertNotEqual(first, b.read_bytes()[6:34])

    def test_temp_folder_is_deleted_after_success(self):
        _, res = self.encrypt("Set-Content -Path (Join-Path $d 'x.txt') -Value 'x'")
        self.assertEqual(res.returncode, 0, res.stderr)
        self.assertEqual(list(self.temp.iterdir()), [])

    def test_failure_mid_run_leaves_no_plaintext_and_no_output(self):
        work = "Set-Content -Path (Join-Path $d 'data.sql') -Value 'secret rows'; throw 'dump failed'"
        target, res = self.encrypt(work)
        self.assertNotEqual(res.returncode, 0)
        self.assertIn("dump failed", res.stderr)
        self.assertEqual(list(self.temp.iterdir()), [])
        self.assertFalse(target.exists())
        leftovers = list(self.out.iterdir()) if self.out.exists() else []
        self.assertEqual(leftovers, [])

    def test_short_or_empty_passphrase_is_refused_before_any_work(self):
        for bad in ("", "short"):
            marker = self.tmp / "ran.txt"
            target, res = self.encrypt(f"Set-Content -Path '{marker}' -Value 'ran'", bad)
            self.assertNotEqual(res.returncode, 0, bad)
            self.assertIn("at least 12", res.stderr)
            self.assertFalse(marker.exists())
            self.assertFalse(target.exists())

    def test_passphrase_comes_from_the_documented_variable_and_is_then_cleared(self):
        code = (f". '{CRYPTO}'; $p = Read-BackupPassphrase; $p; "
                "'env=[' + $env:TAVERN_BACKUP_PASSPHRASE + ']'")
        res = pwsh(code, {"TAVERN_BACKUP_PASSPHRASE": PASSPHRASE})
        self.assertEqual(res.stdout.split(), [PASSPHRASE.split()[0], PASSPHRASE.split()[1],
                                              PASSPHRASE.split()[2], "env=[]"])
        res = pwsh(f". '{CRYPTO}'; Read-BackupPassphrase", {"TAVERN_BACKUP_PASSPHRASE": "short"})
        self.assertNotEqual(res.returncode, 0)

    def test_retention_deletes_only_expired_backups(self):
        out, legacy = self.out, self.tmp / "legacy"
        out.mkdir()
        legacy.mkdir()
        old, new = "tl-backups-20200101-000000.tlenc", "tl-backups-29991231-000000.tlenc"
        keep = ["other.tlenc", "tl-backups-x.txt", "tl-backups-20200101-000000.txt",
                "tl-backups-20261399-000000.tlenc", new]
        for name in [old] + keep:
            (out / name).write_bytes(b"x")
        (legacy / "20200101-000000").mkdir()
        (legacy / "29991231-000000").mkdir()
        (legacy / "notes").mkdir()
        res = pwsh(f". '{CRYPTO}'; Remove-ExpiredBackups '{out}' '{legacy}' 35")
        self.assertEqual(res.returncode, 0, res.stderr)
        self.assertEqual(sorted(p.name for p in out.iterdir()), sorted(keep))
        self.assertEqual(sorted(p.name for p in legacy.iterdir()), ["29991231-000000", "notes"])

    def test_script_takes_no_passphrase_parameter_and_stays_out_of_the_repo(self):
        code = SCRIPT.read_text(encoding="utf-8").split("#>", 1)[1]
        param_block = code.split("param(", 1)[1].split(chr(10) + ")", 1)[0]
        self.assertNotIn("pass", param_block.lower())
        self.assertNotIn("-Passphrase $env", code)
        self.assertNotIn("Set-Content", code.replace("manifest.json", "").split("Invoke-EncryptedBackup")[0])


if __name__ == "__main__":
    unittest.main()
