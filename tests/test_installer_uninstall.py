"""The uninstaller's "delete" checkbox says what it deletes, and does it (T-382)."""

import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DESKTOP = ROOT / "crates" / "desktop"
HOOKS = DESKTOP / "installer" / "hooks.nsh"
ENGLISH = DESKTOP / "installer" / "English.nsh"

# Tauri CLI 2.12.1's English.nsh (package.json pins it): the custom file replaces
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

    def test_the_box_deletes_the_data_folder_unless_the_app_is_updating(self):
        hook = HOOKS.read_text(encoding="utf-8")
        self.assertIn("!macro NSIS_HOOK_POSTUNINSTALL", hook)
        self.assertIn("$DeleteAppDataCheckboxState = 1", hook)
        self.assertIn("$UpdateMode <> 1", hook)
        self.assertEqual(re.findall(r'RmDir /r "([^"]+)"', hook), [r"$APPDATA\TavernLedger"])

    def test_the_hook_never_runs_without_the_checkbox(self):
        # The only RmDir sits inside the checked branch.
        hook = HOOKS.read_text(encoding="utf-8")
        branch = hook.split("${If} $DeleteAppDataCheckboxState = 1", 1)[1].split("${EndIf}", 1)[0]
        self.assertIn("RmDir", branch)
        self.assertNotIn("RmDir", hook.split("${If}", 1)[0].split("!macro", 1)[1])

    def test_the_config_uses_both_files(self):
        nsis = json.loads((DESKTOP / "tauri.conf.json").read_text(encoding="utf-8"))["bundle"]["windows"]["nsis"]
        self.assertEqual(nsis["installerHooks"], "installer/hooks.nsh")
        self.assertEqual(nsis["customLanguageFiles"], {"English": "installer/English.nsh"})
        self.assertTrue((DESKTOP / nsis["installerHooks"]).is_file())


if __name__ == "__main__":
    unittest.main()
