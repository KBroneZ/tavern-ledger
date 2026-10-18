"""Tests for tools/store_package.py: the Store package manifest, folder and exe checks."""

import json
import os
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from xml.etree import ElementTree as ET

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))

import build_icons  # noqa: E402
import store_package as sp  # noqa: E402

NS = {
    "m": "http://schemas.microsoft.com/appx/manifest/foundation/windows10",
    "uap": "http://schemas.microsoft.com/appx/manifest/uap/windows10",
    "uap5": "http://schemas.microsoft.com/appx/manifest/uap/windows10/5",
    "v": "http://schemas.microsoft.com/appx/manifest/virtualization/windows10",
    "rescap": "http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities",
}
REAL = dict(
    name="KBroneZ.TavernLedger",
    publisher="CN=00000000-0000-0000-0000-000000000000",
    publisher_display="KBroneZ",
)
FAKE = dict(
    name=sp.FAKE_NAME,
    publisher=sp.FAKE_PUBLISHER,
    publisher_display=sp.FAKE_PUBLISHER_DISPLAY,
)


def render(variant="full", identity=None, **extra):
    return sp.render_manifest(
        sp.TEMPLATE.read_text(encoding="utf-8"),
        version="1.2.4.0",
        variant=variant,
        **(identity or FAKE),
        **extra,
    )


def png_size(path: Path) -> tuple[int, int]:
    data = path.read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", path
    return struct.unpack(">II", data[16:24])


class ManifestTest(unittest.TestCase):
    def test_both_variants_are_well_formed_xml_with_the_identity_filled_in(self) -> None:
        for variant in sp.VARIANTS:
            root = ET.fromstring(render(variant, REAL))
            identity = root.find("m:Identity", NS)
            self.assertEqual(identity.get("Name"), "KBroneZ.TavernLedger")
            self.assertEqual(identity.get("Publisher"), REAL["publisher"])
            self.assertEqual(identity.get("Version"), "1.2.4.0")
            self.assertEqual(identity.get("ProcessorArchitecture"), "x64")
            display = root.find("m:Properties/m:PublisherDisplayName", NS)
            self.assertEqual(display.text, "KBroneZ")
            app = root.find("m:Applications/m:Application", NS)
            self.assertEqual(app.get("Executable"), "TavernLedger.exe")
            self.assertEqual(app.get("EntryPoint"), "Windows.FullTrustApplication")

    def test_the_full_variant_excludes_only_the_game_folder_and_the_app_data_folder(self) -> None:
        root = ET.fromstring(render("full"))
        path = "m:Properties/v:FileSystemWriteVirtualization/v:ExcludedDirectories/v:ExcludedDirectory"
        excluded = [e.text for e in root.findall(path, NS)]
        self.assertEqual(
            excluded,
            [r"$(KnownFolder:LocalAppData)\Blizzard", r"$(KnownFolder:RoamingAppData)\TavernLedger"],
        )
        caps = [c.get("Name") for c in root.findall("m:Capabilities/rescap:Capability", NS)]
        self.assertEqual(caps, ["runFullTrust", "unvirtualizedResources"])

    def test_the_manual_variant_asks_for_no_restricted_capability_and_excludes_nothing(self) -> None:
        text = render("manual")
        root = ET.fromstring(text)
        caps = [c.get("Name") for c in root.findall("m:Capabilities/rescap:Capability", NS)]
        self.assertEqual(caps, ["runFullTrust"])
        self.assertIsNone(root.find("m:Properties/v:FileSystemWriteVirtualization", NS))
        self.assertNotIn("unvirtualizedResources", text)
        self.assertNotIn("ExcludedDirectory", text)

    def test_no_variant_turns_virtualization_off_for_all_of_appdata(self) -> None:
        # The spike's variant B (desktop6:FileSystemWriteVirtualization=disabled, registry
        # virtualization off) is never offered.
        for variant in sp.VARIANTS:
            text = render(variant)
            self.assertNotIn("desktop6", text)
            self.assertNotIn("RegistryWriteVirtualization", text)
            self.assertNotIn(">disabled<", text)

    def test_the_start_up_task_is_the_one_the_app_asks_for_and_is_off_until_the_player_turns_it_on(self) -> None:
        root = ET.fromstring(render("full"))
        task = root.find(".//uap5:StartupTask", NS)
        self.assertEqual(task.get("TaskId"), sp.startup_task_id())
        self.assertEqual(task.get("Enabled"), "false")
        extension = root.find(".//uap5:Extension", NS)
        self.assertEqual(extension.get("Category"), "windows.startupTask")

    def test_the_description_carries_the_unofficial_fan_notice(self) -> None:
        root = ET.fromstring(render("full"))
        visual = root.find("m:Applications/m:Application/uap:VisualElements", NS)
        notice = "Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment."
        self.assertIn(notice, visual.get("Description"))

    def test_nothing_in_the_manifest_asks_for_more_than_a_full_trust_tracker_needs(self) -> None:
        text = render("full")
        for banned in ("broadFileSystemAccess", "allowElevation", "appExecutionAlias", "documentsLibrary"):
            self.assertNotIn(banned, text)

    def test_every_image_the_manifest_names_exists_in_assets_with_the_size_its_name_says(self) -> None:
        names = sp.ASSET_REF_RE.findall(render("full"))
        self.assertGreaterEqual(len(names), 3)
        for ref in names:
            self.assertTrue((sp.ASSETS / ref.split("\\")[-1]).is_file(), ref)
        self.assertEqual(png_size(sp.ASSETS / "StoreLogo.png"), (50, 50))
        self.assertEqual(png_size(sp.ASSETS / "Square44x44Logo.png"), (44, 44))
        self.assertEqual(png_size(sp.ASSETS / "Square150x150Logo.png"), (150, 150))
        for size in build_icons.STORE_TARGET_SIZES:
            for suffix in ("", "_altform-unplated"):
                path = sp.ASSETS / f"Square44x44Logo.targetsize-{size}{suffix}.png"
                self.assertEqual(png_size(path), (size, size), path.name)

    def test_values_are_escaped_and_a_bad_identity_is_refused(self) -> None:
        text = render("full", dict(REAL, publisher_display="A & B <Fan>"))
        root = ET.fromstring(text)
        self.assertEqual(root.find("m:Properties/m:PublisherDisplayName", NS).text, "A & B <Fan>")
        for bad in (
            dict(REAL, name="ab"),
            dict(REAL, name="has space"),
            dict(REAL, name="x" * 51),
            dict(REAL, publisher="TavernLedger"),
            dict(REAL, publisher='CN="><x'),
            dict(REAL, publisher_display="  "),
        ):
            with self.assertRaises(sp.Refused, msg=str(bad)):
                render("full", bad)

    def test_the_version_must_be_a_store_version(self) -> None:
        template = sp.TEMPLATE.read_text(encoding="utf-8")
        for bad in ("0.2.4.0", "1.2.4.1", "1.2.4", "1.2.3.4.0", "65536.0.0.0", "1.65536.0.0", "a.b.c.0", "1.2.4.0\n"):
            with self.assertRaises(sp.Refused, msg=bad):
                sp.render_manifest(template, version=bad, variant="full", **FAKE)

    def test_unknown_variants_and_template_fields_are_refused(self) -> None:
        with self.assertRaises(sp.Refused):
            render("b")
        with self.assertRaises(sp.Refused):
            sp.render_manifest("<x>{{NOPE}}</x>", version="1.0.0.0", variant="full", **FAKE)


class SubmissionTest(unittest.TestCase):
    def test_a_package_for_submission_refuses_the_local_test_identity(self) -> None:
        partly_fake = (
            FAKE,
            dict(REAL, name=sp.FAKE_NAME),
            dict(REAL, publisher=sp.FAKE_PUBLISHER),
            dict(REAL, publisher_display=sp.FAKE_PUBLISHER_DISPLAY),
        )
        for identity in partly_fake:
            with self.assertRaises(sp.Refused, msg=str(identity)):
                render("full", identity, for_submission=True)

    def test_a_real_identity_is_accepted_for_submission_but_a_test_alias_is_not(self) -> None:
        self.assertIn("KBroneZ.TavernLedger", render("full", REAL, for_submission=True))
        with self.assertRaises(sp.Refused):
            render("full", REAL, for_submission=True, test_alias="TLTest.exe")

    def test_the_local_test_identity_is_clearly_fake(self) -> None:
        self.assertTrue(sp.is_fake(**FAKE))
        self.assertTrue(sp.is_fake(**dict(REAL, name=sp.FAKE_NAME.upper())), "case does not matter")
        self.assertFalse(sp.is_fake(**REAL))
        self.assertIn("LocalTest", sp.FAKE_NAME)
        self.assertIn("LocalTest", sp.FAKE_PUBLISHER)

    def test_the_test_alias_is_only_added_when_asked_and_is_checked(self) -> None:
        self.assertNotIn("appExecutionAlias", render("full"))
        root = ET.fromstring(render("full", test_alias="TLTest.exe"))
        self.assertEqual(root.find(".//uap5:ExecutionAlias", NS).get("Alias"), "TLTest.exe")
        for bad in ("x", "bad name.exe", "a.exe;", "TLTest"):
            with self.assertRaises(sp.Refused, msg=bad):
                render("full", test_alias=bad)


class StageTest(unittest.TestCase):
    def test_a_folder_that_is_not_a_package_folder_is_never_deleted(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            exe = Path(tmp) / "build.exe"
            exe.write_bytes(b"MZ")
            precious = Path(tmp) / "documents"
            precious.mkdir()
            (precious / "keep.txt").write_text("mine")
            with self.assertRaises(sp.Refused):
                sp.stage(exe, precious, manifest=render("full"))
            self.assertTrue((precious / "keep.txt").is_file())

    def test_the_package_folder_has_the_exe_the_images_and_the_manifest_and_nothing_else(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            exe = Path(tmp) / "build.exe"
            exe.write_bytes(b"MZ" + sp.STORE_MARK)
            out = Path(tmp) / "pkg"
            out.mkdir()
            (out / "AppxManifest.xml").write_text("an older package folder")
            (out / "stale.txt").write_text("left over")
            files = sp.stage(exe, out, manifest=render("full"))
            self.assertIn("TavernLedger.exe", files)
            self.assertIn("AppxManifest.xml", files)
            self.assertIn("Assets/StoreLogo.png", files)
            self.assertNotIn("stale.txt", files, "the folder is emptied first")
            self.assertEqual((out / "TavernLedger.exe").read_bytes(), exe.read_bytes())
            known = ("TavernLedger.exe", "AppxManifest.xml")
            self.assertTrue(all(f in known or f.startswith("Assets/") for f in files))

    def test_a_manifest_that_names_a_missing_image_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            exe = Path(tmp) / "build.exe"
            exe.write_bytes(b"MZ")
            empty = Path(tmp) / "no-assets"
            empty.mkdir()
            with self.assertRaises(sp.Refused):
                sp.stage(exe, Path(tmp) / "pkg", manifest=render("full"), assets=empty)

    def test_a_missing_exe_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(sp.Refused):
                sp.stage(Path(tmp) / "none.exe", Path(tmp) / "pkg", manifest=render("full"))


class ExeCheckTest(unittest.TestCase):
    KEY = b"dW50cnVzdGVkIGNvbW1lbnQ6IA=="

    def check(self, data: bytes, **kwargs) -> list[str]:
        return sp.check_exe(data, key=self.KEY, **kwargs)

    def test_a_store_exe_without_the_updater_passes(self) -> None:
        self.assertEqual(self.check(b"MZ" + sp.STORE_MARK + b"something else"), [])

    def test_the_installer_exe_is_refused(self) -> None:
        problems = self.check(b"MZ" + sp.INSTALLER_MARK)
        self.assertTrue(any("without the `store` feature" in p for p in problems))
        self.assertTrue(any("installer edition" in p for p in problems))

    def test_an_exe_with_the_updater_in_it_is_refused(self) -> None:
        self.assertTrue(self.check(b"MZ" + sp.STORE_MARK + sp.UPDATER_ENDPOINT))
        self.assertTrue(self.check(b"MZ" + sp.STORE_MARK + self.KEY))

    def test_the_committed_updater_key_is_the_one_checked(self) -> None:
        key = sp.updater_key()
        self.assertTrue(key.startswith(b"dW50cnVzdGVkIGNvbW1lbnQ6"))
        self.assertTrue(sp.check_exe(b"MZ" + sp.STORE_MARK + key, key=key))


class StoreConfigTest(unittest.TestCase):
    def test_the_store_config_override_removes_the_updater_and_its_artifacts(self) -> None:
        conf = json.loads((ROOT / "crates" / "desktop" / "tauri.store.conf.json").read_text(encoding="utf-8"))
        # JSON merge patch: null removes the key.
        self.assertIsNone(conf["plugins"]["updater"])
        self.assertFalse(conf["bundle"]["createUpdaterArtifacts"])
        main = json.loads(sp.CONF.read_text(encoding="utf-8"))
        self.assertNotEqual(conf["identifier"], main["identifier"], "its own WebView2 folder")


class PackagingScriptTest(unittest.TestCase):
    SCRIPT = (ROOT / "tools" / "make-msix.ps1").read_text(encoding="utf-8")

    def test_the_script_never_signs_uploads_or_submits(self) -> None:
        for banned in ("signtool", "Invoke-WebRequest", "Invoke-RestMethod", "gh release", "gh pr", "Set-AuthenticodeSignature", "New-SelfSignedCertificate"):
            self.assertNotIn(banned, self.SCRIPT)

    def test_the_script_builds_the_store_edition_into_its_own_target_folder(self) -> None:
        self.assertIn("tauri.store.conf.json", self.SCRIPT)
        self.assertIn("'store,store-manual-setup'", self.SCRIPT)
        self.assertIn(r"$TargetDir = 'target\store'", self.SCRIPT)
        # The exe is checked before anything is packed.
        self.assertLess(
            self.SCRIPT.index("Invoke-Native 'the exe check'"),
            self.SCRIPT.index("Invoke-Native 'makeappx pack'"),
        )

    def test_the_workflow_passes_the_submission_switch_in_a_way_that_reaches_the_script(self) -> None:
        # Splatting an ARRAY passes '-ForSubmission' as a plain string and the switch is
        # silently off; only a hashtable binds it by name.
        workflow = (ROOT / ".github" / "workflows" / "store-package.yml").read_text(encoding="utf-8")
        self.assertNotIn("$extra = @()", workflow)
        self.assertIn("$extra = @{}", workflow)
        self.assertIn("$extra.ForSubmission = $true", workflow)

    @unittest.skipUnless(shutil.which("pwsh"), "PowerShell 7 is not installed")
    def test_for_submission_with_the_local_test_identity_stops_before_anything_is_packed(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "out"
            # No -Exe: if the refusal did not come first, this would start a real build.
            run = subprocess.run(
                ["pwsh", "-NoProfile", "-File", str(ROOT / "tools" / "make-msix.ps1"),
                 "-ForSubmission", "-OutDir", str(out), "-IdentityName", ""],
                capture_output=True, text=True, check=False, env={k: v for k, v in os.environ.items() if not k.startswith("STORE_")},
            )
            self.assertNotEqual(run.returncode, 0)
            self.assertIn("real identity", run.stdout + run.stderr)
            self.assertEqual(list(out.glob("*.msix")), [])

    def test_a_package_without_the_real_identity_says_so_in_its_file_name(self) -> None:
        self.assertIn("_fake-identity", self.SCRIPT)

    def test_a_local_test_package_cannot_be_a_submission(self) -> None:
        self.assertIn("-LocalTest and -ForSubmission cannot be combined", self.SCRIPT)


if __name__ == "__main__":
    unittest.main()
