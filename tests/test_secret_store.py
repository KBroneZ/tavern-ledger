"""Tests for the Supabase secret key store (T-570): tools/secret_store.ps1 and
tools/supabase_secret.ps1.

Only a fake key is used. The store tests need Windows (DPAPI) and pwsh 7; the
env-file tests need pwsh 7 only. Network calls are replaced by a stand-in
Invoke-WebRequest, so nothing leaves the machine.
"""

import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
STORE = ROOT / "tools" / "secret_store.ps1"
COMMAND = ROOT / "tools" / "supabase_secret.ps1"
READERS = [ROOT / "tools" / "backup_supabase.ps1", ROOT / "tools" / "sim_consented_games.ps1"]
PWSH = shutil.which("pwsh")
WINDOWS = sys.platform == "win32"

URL = "https://abcdefghijklmnop.supabase.co"
KEY = "sb_secret_fake-key-for-tests"
OTHER_KEY = "sb_secret_another-fake-key"

# Stand-in for the network: records the call and answers like the hosted
# Storage API does for the private games bucket, or with $env:FAKE_STATUS.
FAKE_WEB = r"""
function Invoke-WebRequest {
    param($Method, $Uri, $Headers, $TimeoutSec, $MaximumRedirection, [switch]$SkipHttpErrorCheck)
    $names = ($Headers.Keys | Sort-Object) -join ','
    Add-Content -LiteralPath $env:FAKE_CALLS -Value "$Method $Uri headers=$names timeout=$TimeoutSec redirects=$MaximumRedirection key=$($Headers.apikey -ceq $env:FAKE_EXPECT)"
    if ($env:FAKE_THROW) { throw "connection reset while sending apikey $($Headers.apikey)" }
    $status = if ($env:FAKE_STATUS) { [int]$env:FAKE_STATUS } else { 200 }
    $body = if ($env:FAKE_BODY) { $env:FAKE_BODY } else { '{"id":"games","name":"games","public":false}' }
    [pscustomobject]@{ StatusCode = $status; Content = $body }
}
"""


def pwsh(code, env_extra=None):
    env = dict(os.environ)
    env.update(env_extra or {})
    return subprocess.run(
        [PWSH, "-NoProfile", "-NonInteractive", "-Command", code],
        capture_output=True, text=True, timeout=120, env=env, cwd=ROOT,
    )


def q(path):
    """A PowerShell single-quoted literal."""
    return "'" + str(path).replace("'", "''") + "'"


@unittest.skipUnless(PWSH, "pwsh 7 not installed")
class EnvFileTest(unittest.TestCase):
    """Parsing the old plaintext file as data."""

    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="tl-secret-"))
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)

    def parse(self, text):
        env_file = self.tmp / "supabase.env"
        env_file.write_text(text, encoding="utf-8")
        code = (f". {q(STORE)}; $c = Read-SupabaseEnvFile {q(env_file)}; "
                "'url=' + $c.Url; 'key-ok=' + ($c.Key -ceq $env:FAKE_EXPECT)")
        return pwsh(code, {"FAKE_EXPECT": KEY})

    def assert_no_secret(self, res):
        self.assertNotIn(KEY, res.stdout + res.stderr)
        self.assertNotIn("fake-key", res.stdout + res.stderr)

    def test_reads_the_two_variables(self):
        for text in (f"SUPABASE_URL={URL}\nSUPABASE_SECRET_KEY={KEY}\n",
                     f'\n  SUPABASE_URL = "{URL}"  \r\nSUPABASE_SECRET_KEY="{KEY}"\r\n\n'):
            res = self.parse(text)
            self.assertEqual(res.returncode, 0, res.stderr)
            self.assertEqual(res.stdout.split(), [f"url={URL}", "key-ok=True"])
            self.assert_no_secret(res)

    def test_other_content_stops_it_naming_only_the_variable_or_line(self):
        cases = [
            (f"SUPABASE_URL={URL}\nSUPABASE_SECRET_KEY={KEY}\nSUPABASE_DB_PASSWORD={KEY}\n",
             "SUPABASE_DB_PASSWORD"),
            (f"SUPABASE_URL={URL}\nSUPABASE_SECRET_KEY={KEY}\n# old {KEY}\n", "Line 3"),
            (f"SUPABASE_URL={URL}\n{KEY}\n", "Line 2"),
            (f"SUPABASE_URL={URL}\nSUPABASE_SECRET_KEY={KEY}\nSUPABASE_SECRET_KEY={OTHER_KEY}\n",
             "twice"),
            (f"export SUPABASE_URL={URL}\nSUPABASE_SECRET_KEY={KEY}\n", "Line 1"),
        ]
        for text, expected in cases:
            res = self.parse(text)
            self.assertNotEqual(res.returncode, 0, text)
            self.assertIn(expected, res.stderr)
            self.assert_no_secret(res)

    def test_missing_legacy_or_malformed_values_are_refused(self):
        cases = [
            (f"SUPABASE_URL={URL}\n", "no SUPABASE_SECRET_KEY"),
            (f"SUPABASE_URL={URL}\nSUPABASE_SECRET_KEY=eyJhbGciOi.legacy.jwt\n", "sb_secret_"),
            (f"SUPABASE_URL={URL}\nSUPABASE_SECRET_KEY={KEY} trailing\n", "unexpected characters"),
            (f"SUPABASE_URL=http://example.com\nSUPABASE_SECRET_KEY={KEY}\n", "supabase.co"),
        ]
        for text, expected in cases:
            res = self.parse(text)
            self.assertNotEqual(res.returncode, 0, text)
            self.assertIn(expected, res.stderr)
            self.assert_no_secret(res)

    def test_the_file_is_never_executed(self):
        marker = self.tmp / "ran.txt"
        text = f"SUPABASE_URL={URL}\nSUPABASE_SECRET_KEY={KEY}\n$(Set-Content '{marker}' x)\n"
        res = self.parse(text)
        self.assertNotEqual(res.returncode, 0)
        self.assertFalse(marker.exists())


@unittest.skipUnless(PWSH and WINDOWS, "needs Windows DPAPI and pwsh 7")
class StoreTest(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="tl-secret-"))
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        self.store = self.tmp / "store" / "supabase-secret.json"
        self.env_file = self.tmp / "supabase.env"
        self.env_file.write_text(f"SUPABASE_URL={URL}\nSUPABASE_SECRET_KEY={KEY}\n", encoding="utf-8")
        self.calls = self.tmp / "calls.txt"

    def run_store(self, code, extra=None):
        env = {"FAKE_CALLS": str(self.calls), "FAKE_EXPECT": KEY}
        env.update(extra or {})
        return pwsh(f". {q(STORE)}; {FAKE_WEB}; {code}", env)

    def run_command(self, args, extra=None, prelude=""):
        env = {"FAKE_CALLS": str(self.calls), "FAKE_EXPECT": KEY}
        env.update(extra or {})
        env_file = "" if "-Check" in args or "-SetKey" in args else f"-EnvFile {q(self.env_file)}"
        code = f"{FAKE_WEB}; {prelude}; & {q(COMMAND)} {env_file} -StorePath {q(self.store)} {args}"
        return pwsh(code, env)

    def assert_no_secret(self, res):
        self.assertNotIn(KEY, res.stdout + res.stderr)
        self.assertNotIn("fake-key", res.stdout + res.stderr)

    def calls_made(self):
        return self.calls.read_text(encoding="utf-8").splitlines() if self.calls.exists() else []

    def test_migration_stores_the_key_encrypted_and_checks_it(self):
        res = self.run_command("")
        self.assertEqual(res.returncode, 0, res.stderr)
        self.assert_no_secret(res)
        raw = self.store.read_bytes()
        self.assertNotIn(KEY.encode(), raw)
        self.assertNotIn(b"fake-key", raw)
        self.assertIn(URL.encode(), raw)
        self.assertTrue(self.env_file.exists(), "the plaintext file is kept until -DeletePlaintext")
        self.assertEqual(self.calls_made(), [
            f"Get {URL}/storage/v1/bucket/games headers=apikey timeout=15 redirects=0 key=True"])
        self.assertIn("Check passed", res.stdout)
        self.assertEqual([p.name for p in self.store.parent.iterdir()], [self.store.name])

    def test_readers_get_the_key_from_the_store(self):
        self.assertEqual(self.run_command("").returncode, 0)
        res = self.run_store(f"$c = Get-SupabaseConnection -StorePath {q(self.store)}; "
                             "'url=' + $c.Url; 'key-ok=' + ($c.Key -ceq $env:FAKE_EXPECT)")
        self.assertEqual(res.returncode, 0, res.stderr)
        self.assertEqual(res.stdout.split(), [f"url={URL}", "key-ok=True"])

    def test_no_store_is_an_error_and_never_falls_back_to_the_plaintext_file(self):
        res = self.run_store(f"Get-SupabaseConnection -StorePath {q(self.store)}")
        self.assertNotEqual(res.returncode, 0)
        self.assertIn("supabase_secret.ps1", res.stderr)
        self.assert_no_secret(res)

    def test_rerun_is_a_no_op(self):
        self.assertEqual(self.run_command("").returncode, 0)
        before = self.store.read_bytes()
        res = self.run_command("")
        self.assertEqual(res.returncode, 0, res.stderr)
        self.assertIn("already stored", res.stdout)
        self.assertEqual(self.store.read_bytes(), before)

    def test_a_different_stored_key_is_never_overwritten(self):
        self.assertEqual(self.run_command("").returncode, 0)
        before = self.store.read_bytes()
        self.env_file.write_text(f"SUPABASE_URL={URL}\nSUPABASE_SECRET_KEY={OTHER_KEY}\n", encoding="utf-8")
        res = self.run_command("-DeletePlaintext -Confirm:$false")
        self.assertNotEqual(res.returncode, 0)
        self.assertIn("not overwriting", res.stderr)
        self.assertEqual(self.store.read_bytes(), before)
        self.assertTrue(self.env_file.exists())
        self.assertNotIn("another-fake", res.stdout + res.stderr)

    def test_an_interrupted_write_leaves_nothing_usable_and_is_swept(self):
        self.store.parent.mkdir(parents=True)
        partial = self.store.parent / "supabase-secret.0123.partial"
        partial.write_text("{half", encoding="utf-8")
        res = self.run_command("")
        self.assertEqual(res.returncode, 0, res.stderr)
        self.assertFalse(partial.exists())
        self.assertTrue(self.store.exists())

    def test_a_broken_store_is_reported_not_replaced(self):
        self.store.parent.mkdir(parents=True)
        self.store.write_text('{"format":"tl-supabase-secret-1","url":"%s","key_dpapi":"AAAA"}' % URL,
                              encoding="utf-8")
        before = self.store.read_bytes()
        res = self.run_command("")
        self.assertNotEqual(res.returncode, 0)
        self.assertIn("could not be decrypted", res.stderr)
        self.assertEqual(self.store.read_bytes(), before)

    def test_a_failed_check_deletes_nothing(self):
        for status, body in (("401", '{"error":"Unauthorized"}'),
                             ("200", '{"id":"games","name":"games","public":true}')):
            res = self.run_command("-DeletePlaintext -Confirm:$false",
                                   {"FAKE_STATUS": status, "FAKE_BODY": body})
            self.assertNotEqual(res.returncode, 0, status)
            self.assertTrue(self.env_file.exists())
            self.assert_no_secret(res)

    def test_delete_plaintext_removes_only_the_env_file_after_the_checks(self):
        res = self.run_command("-DeletePlaintext -Confirm:$false")
        self.assertEqual(res.returncode, 0, res.stderr)
        self.assertFalse(self.env_file.exists())
        self.assertTrue(self.store.exists())
        self.assert_no_secret(res)
        # Afterwards the command only checks the stored key.
        res = self.run_command("")
        self.assertEqual(res.returncode, 0, res.stderr)
        self.assertIn("Check passed", res.stdout)

    def test_delete_plaintext_refuses_a_link(self):
        real = self.tmp / "real.env"
        real.write_text(self.env_file.read_text(encoding="utf-8"), encoding="utf-8")
        self.env_file.unlink()
        try:
            os.symlink(real, self.env_file)
        except OSError:
            self.skipTest("symbolic links need Developer Mode")
        res = self.run_command("-DeletePlaintext -Confirm:$false")
        self.assertNotEqual(res.returncode, 0)
        self.assertIn("link", res.stderr)
        self.assertTrue(real.exists())

    def test_a_failed_request_never_shows_the_key(self):
        res = self.run_command("", {"FAKE_THROW": "1"})
        self.assertNotEqual(res.returncode, 0)
        self.assertIn("connection reset", res.stderr)
        self.assert_no_secret(res)

    def test_without_confirmation_nothing_is_deleted_or_replaced(self):
        # -NonInteractive cannot answer the prompt, so ShouldProcess must refuse.
        res = self.run_command("-DeletePlaintext")
        self.assertNotEqual(res.returncode, 0)
        self.assertTrue(self.env_file.exists())
        before = self.store.read_bytes()
        fake_prompt = ("function Read-Host { param($Prompt, [switch]$AsSecureString) "
                       "ConvertTo-SecureString $env:FAKE_NEW_KEY -AsPlainText -Force }")
        res = self.run_command("-SetKey", {"FAKE_NEW_KEY": OTHER_KEY, "FAKE_EXPECT": OTHER_KEY},
                               fake_prompt)
        self.assertNotEqual(res.returncode, 0)
        self.assertEqual(self.store.read_bytes(), before)

    def test_what_if_is_refused_before_anything_is_written(self):
        res = self.run_command("-WhatIf")
        self.assertNotEqual(res.returncode, 0)
        self.assertIn("-WhatIf is not supported", res.stderr)
        self.assertFalse(self.store.parent.exists())
        self.assertEqual(self.calls_made(), [])

    def test_links_and_junctions_are_recognised(self):
        target = self.tmp / "target"
        target.mkdir()
        junction = self.tmp / "junction"
        made = subprocess.run(["cmd", "/c", "mklink", "/J", str(junction), str(target)],
                              capture_output=True, text=True)
        self.assertEqual(made.returncode, 0, made.stderr)
        res = self.run_store(f"Test-IsLink {q(junction)}; Test-IsLink {q(target)}; "
                             f"Test-IsLink {q(self.env_file)}")
        self.assertEqual(res.returncode, 0, res.stderr)
        self.assertEqual(res.stdout.split(), ["True", "False", "False"])

    def test_check_only(self):
        self.assertEqual(self.run_command("").returncode, 0)
        self.calls.unlink()
        res = self.run_command("-Check")
        self.assertEqual(res.returncode, 0, res.stderr)
        self.assertEqual(len(self.calls_made()), 1)

    def test_set_key_after_a_rotation_checks_the_new_key_then_replaces(self):
        self.assertEqual(self.run_command("").returncode, 0)
        before = self.store.read_bytes()
        fake_prompt = ("function Read-Host { param($Prompt, [switch]$AsSecureString) "
                       "ConvertTo-SecureString $env:FAKE_NEW_KEY -AsPlainText -Force }")
        res = self.run_command("-SetKey -Confirm:$false",
                               {"FAKE_NEW_KEY": OTHER_KEY, "FAKE_EXPECT": OTHER_KEY}, fake_prompt)
        self.assertEqual(res.returncode, 0, res.stderr)
        self.assertNotEqual(self.store.read_bytes(), before)
        self.assertNotIn(b"another-fake", self.store.read_bytes())
        self.assertNotIn("another-fake", res.stdout + res.stderr)
        res = self.run_store(f"$c = Get-SupabaseConnection -StorePath {q(self.store)}; "
                             "'key-ok=' + ($c.Key -ceq $env:FAKE_EXPECT)", {"FAKE_EXPECT": OTHER_KEY})
        self.assertEqual(res.stdout.split(), ["key-ok=True"])

    def test_set_key_keeps_the_old_key_when_the_new_one_fails(self):
        self.assertEqual(self.run_command("").returncode, 0)
        before = self.store.read_bytes()
        fake_prompt = ("function Read-Host { param($Prompt, [switch]$AsSecureString) "
                       "ConvertTo-SecureString $env:FAKE_NEW_KEY -AsPlainText -Force }")
        res = self.run_command("-SetKey -Confirm:$false",
                               {"FAKE_NEW_KEY": OTHER_KEY, "FAKE_STATUS": "401"}, fake_prompt)
        self.assertNotEqual(res.returncode, 0)
        self.assertEqual(self.store.read_bytes(), before)


@unittest.skipUnless(PWSH, "pwsh 7 not installed")
class ReadersTest(unittest.TestCase):
    def test_readers_use_the_store_only(self):
        for script in READERS:
            code = script.read_text(encoding="utf-8").split("#>", 1)[1]
            self.assertIn("secret_store.ps1", code, script.name)
            self.assertIn("Get-SupabaseConnection", code, script.name)
            self.assertNotIn("supabase.env", code, script.name)
            self.assertNotIn("SUPABASE_SECRET_KEY", code, script.name)


if __name__ == "__main__":
    unittest.main()
