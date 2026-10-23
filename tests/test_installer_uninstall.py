"""The uninstaller's "delete" checkbox says what it deletes, and does it (T-382)."""

import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DESKTOP = ROOT / "crates" / "desktop"
HOOKS = DESKTOP / "installer" / "hooks.nsh"
ENGLISH = DESKTOP / "installer" / "English.nsh"

# Tauri CLI 2.12.2's English.nsh (package.json pins it): the custom file replaces
# it whole, so every string must be there. A CLI upgrade that adds one fails
# this test, and the file is refreshed from the CLI's own English.nsh.
TAURI_KEYS = {
    "addOrReinstall", "alreadyInstalled", "alreadyInstalledLong", "appRunning",
    "appRunningOkKill", "chooseMaintenanceOption", "choowHowToInstall",
    "createDesktop", "dontUninstall", "dontUninstallDowngrade", "failedToKillApp",
    "installingWebview2", "newerVersionInstalled", "older",
    "olderOrUnknownVersionInstalled", "silentDowngrades", "unableToUninstall",
    "uninstallApp", "uninstallBeforeInstalling", "unknown", "webview2AbortError",
    "webview2DownloadError", "webview2DownloadSuccess", "webview2Downloading",
    "webview2InstallError", "webview2InstallSuccess", "deleteAppData",
}
# The checkbox is one line, 400 px wide at 96 dpi in Tauri's template.
MAX_LABEL = 60


class UninstallerTest(unittest.TestCase):
    def strings(self):
        found = re.findall(
            r'^LangString (\w+) \$\{LANG_ENGLISH\} "(.*)"$',
            ENGLISH.read_text(encoding="utf-8"),
            re.M,
        )
        return dict(found)

    def test_the_custom_english_file_has_every_string_tauri_needs(self):
        self.assertEqual(set(self.strings()), TAURI_KEYS)

    def test_the_label_says_what_the_box_deletes_and_fits_one_line(self):
        label = self.strings()["deleteAppData"]
        self.assertEqual(label, "Also delete my games and settings (cannot be undone)")
        self.assertLessEqual(len(label), MAX_LABEL)

    def code(self):
        """The hook without its comments, and the body of the macro."""
        hook = HOOKS.read_text(encoding="utf-8")
        lines = [line for line in hook.splitlines() if not line.lstrip().startswith(";")]
        return "\n".join(lines).split("!macro NSIS_HOOK_POSTUNINSTALL", 1)[1].split("!macroend", 1)[0]

    def test_the_box_deletes_the_data_folder_only_when_ticked_and_not_updating(self):
        body = self.code()
        # The exact conjunction, on adjacent lines: an `OrIf` or a dropped line would fail here.
        self.assertRegex(
            body,
            r"\$\{If\} \$DeleteAppDataCheckboxState = 1\n\s*\$\{AndIf\} \$UpdateMode <> 1\n",
        )
        self.assertNotIn("OrIf", body)
        self.assertEqual(body.count("${If} $DeleteAppDataCheckboxState"), 1)

    def test_the_deletion_is_scoped_to_the_apps_own_folder_and_never_nsis_recursive(self):
        body = self.code()
        self.assertNotIn("RmDir /r", body, "NSIS's recursive delete follows junctions")
        self.assertEqual(
            re.findall(r'rd /s /q "([^"]+)"', body), [r"$APPDATA\TavernLedger"], "one deletion, in the user's AppData"
        )
        self.assertRegex(body, r'RmDir "\$APPDATA\\TavernLedger"')  # a link at the root: only the link
        self.assertIn("GetFileAttributesW", body)
        self.assertIn("0x400", body)  # FILE_ATTRIBUTE_REPARSE_POINT
        self.assertIn('$APPDATA != ""', body)  # never a path from the drive's root

    def test_every_deletion_sits_inside_the_checked_not_updating_branch(self):
        body = self.code()
        deletions = [m.start() for m in re.finditer(r"RmDir|nsExec::Exec", body)]
        self.assertEqual(len(deletions), 2, "the link-only RmDir and the rd command")
        opening = body.index("${If} $DeleteAppDataCheckboxState = 1")
        depth, closing = 0, None
        for match in re.finditer(r"\$\{(If|EndIf)\}", body):
            depth += 1 if match.group(1) == "If" else -1
            if match.start() >= opening and depth == 0:
                closing = match.start()
                break
        self.assertIsNotNone(closing)
        for at in deletions:
            self.assertTrue(opening < at < closing, "a deletion outside the branch")

    def test_the_custom_strings_follow_the_pinned_cli(self):
        # The fixture above is the string table of this exact CLI: a new CLI may
        # add strings, and the custom file replaces the table whole.
        package = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
        pinned = package["devDependencies"]["@tauri-apps/cli"]
        self.assertEqual(pinned, "2.12.2", "refresh English.nsh and TAURI_KEYS from the new CLI")

    def test_the_config_uses_both_files(self):
        nsis = json.loads((DESKTOP / "tauri.conf.json").read_text(encoding="utf-8"))["bundle"]["windows"]["nsis"]
        self.assertEqual(nsis["installerHooks"], "installer/hooks.nsh")
        self.assertEqual(nsis["customLanguageFiles"], {"English": "installer/English.nsh"})
        self.assertTrue((DESKTOP / nsis["installerHooks"]).is_file())


if __name__ == "__main__":
    unittest.main()
