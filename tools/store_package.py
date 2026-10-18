"""Helpers for the Microsoft Store package (T-379, D-158). Standard library only.

tools/make-msix.ps1 builds the Store edition and calls these to write the
package manifest, lay out the package folder and check the exe before
`makeappx pack` turns the folder into an unsigned .msix. Nothing here signs,
uploads or submits anything.

Usage:
    python tools/store_package.py stage --exe EXE --out DIR [--variant full|manual]
        [--name NAME --publisher CN=... --publisher-display TEXT]
        [--version A.B.C.D] [--for-submission] [--test-alias NAME.exe]
        Writes DIR (emptied first; refused if it holds something else): the exe, Assets/ and
        AppxManifest.xml. The exe is checked as `check-exe` does.
    python tools/store_package.py check-identity [the same identity options as stage]
        Exit 1 if the identity, version or variant cannot make a package as asked
        (so a refusal comes before a long build, not after it).
    python tools/store_package.py check-exe EXE
        Exit 1 unless EXE is the Store edition, with no updater in it.

The identity (name, publisher, publisher display name) comes from Partner
Center once the account exists. Until then the defaults are clearly fake, and
`--for-submission` refuses them, so a package made with them can never be
mistaken for one to submit.

Variants (the manifest differs in two blocks):
    full    the manifest excludes `%LOCALAPPDATA%\\Blizzard` and
            `%APPDATA%\\TavernLedger` from virtualization and asks for the
            `unvirtualizedResources` capability (restricted: Microsoft decides
            at certification). The Fix button writes the game's log.config
            where the game reads it, and the history lives in the same folder
            as the installer edition's.
    manual  no such capability and no excluded folders. Build the app with the
            `store-manual-setup` feature too: no Fix button, and the history
            stays in the package's private store (uninstalling the Store app
            deletes it).
The spike's variant that turned off virtualization for all of AppData is not
offered.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import sys
from pathlib import Path
from xml.sax.saxutils import escape

import store_version

ROOT = Path(__file__).resolve().parent.parent
STORE_DIR = ROOT / "crates" / "desktop" / "store"
TEMPLATE = STORE_DIR / "AppxManifest.xml.template"
ASSETS = STORE_DIR / "Assets"
CONF = ROOT / "crates" / "desktop" / "tauri.conf.json"
STARTUP_RS = ROOT / "crates" / "desktop" / "src" / "startup.rs"
EXE_NAME = "TavernLedger.exe"

FAKE_NAME = "TavernLedger.LocalTest"
FAKE_PUBLISHER = "CN=TavernLedgerLocalTest"
FAKE_PUBLISHER_DISPLAY = "Tavern Ledger (local test)"

NAME_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9.\-]{2,49}")
PUBLISHER_RE = re.compile(r"CN=[^<>&\"\r\n]+")
VERSION_RE = re.compile(r"([1-9][0-9]{0,4})\.([0-9]{1,5})\.([0-9]{1,5})\.0")
ALIAS_RE = re.compile(r"[A-Za-z0-9]{3,40}\.exe")
FIELD_RE = re.compile(r"\{\{([A-Z_]+)\}\}")
ASSET_REF_RE = re.compile(r'[>"](Assets\\[^"<]+\.png)[<"]')
MAX_PART = 65535

VARIANTS = ("full", "manual")

# The folders the `full` variant excludes from virtualization. The first is
# where the game reads log.config; the second is the app's own data folder.
EXCLUDED_DIRECTORIES = (
    r"$(KnownFolder:LocalAppData)\Blizzard",
    r"$(KnownFolder:RoamingAppData)\TavernLedger",
)

# Strings that must be in the Store edition's exe, and strings that must not.
STORE_MARK = b"microsoft-store-edition"
INSTALLER_MARK = b"installer-edition"
# The updater's endpoint, and (read from the config) its public key.
UPDATER_ENDPOINT = b"releases/latest/download"


class Refused(ValueError):
    """The package cannot be made as asked (said why)."""


def is_fake(name: str, publisher: str, publisher_display: str) -> bool:
    """Whether any of the three is one of the local-test values."""
    fakes = {FAKE_NAME.lower(), FAKE_PUBLISHER.lower(), FAKE_PUBLISHER_DISPLAY.lower()}
    return any(value.strip().lower() in fakes for value in (name, publisher, publisher_display))


def validate(name: str, publisher: str, publisher_display: str, version: str) -> None:
    if not NAME_RE.fullmatch(name):
        raise Refused(f"identity name {name!r}: 3 to 50 letters, digits, dots and dashes")
    if not PUBLISHER_RE.fullmatch(publisher):
        raise Refused(f"publisher {publisher!r} must look like CN=...")
    if not publisher_display.strip() or len(publisher_display) > 256:
        raise Refused("the publisher display name is empty or too long")
    found = VERSION_RE.fullmatch(version)
    if not found or any(int(part) > MAX_PART for part in found.groups()):
        raise Refused(f"version {version!r}: first part 1 or more, last part 0, each at most {MAX_PART}")


def virtualization_block(variant: str) -> str:
    if variant == "manual":
        return ""
    excluded = "".join(
        f"\n        <virtualization:ExcludedDirectory>{escape(path)}</virtualization:ExcludedDirectory>"
        for path in EXCLUDED_DIRECTORIES
    )
    return (
        "\n    <virtualization:FileSystemWriteVirtualization>"
        "\n      <virtualization:ExcludedDirectories>"
        f"{excluded}"
        "\n      </virtualization:ExcludedDirectories>"
        "\n    </virtualization:FileSystemWriteVirtualization>"
    )


def capabilities_block(variant: str) -> str:
    if variant == "manual":
        return ""
    return '\n    <rescap:Capability Name="unvirtualizedResources" />'


def alias_block(alias: str | None) -> str:
    if alias is None:
        return ""
    if not ALIAS_RE.fullmatch(alias):
        raise Refused(f"test alias {alias!r}: letters and digits, then .exe")
    return (
        f'        <uap5:Extension Category="windows.appExecutionAlias" Executable="{EXE_NAME}" EntryPoint="Windows.FullTrustApplication">\n'
        f'          <uap5:AppExecutionAlias><uap5:ExecutionAlias Alias="{alias}" /></uap5:AppExecutionAlias>\n'
        "        </uap5:Extension>\n"
    )


def render_manifest(
    template: str,
    *,
    name: str,
    publisher: str,
    publisher_display: str,
    version: str,
    variant: str,
    test_alias: str | None = None,
    for_submission: bool = False,
) -> str:
    """The manifest text, from the template. Every field is filled or it is an error."""
    if variant not in VARIANTS:
        raise Refused(f"variant {variant!r}: one of {', '.join(VARIANTS)}")
    validate(name, publisher, publisher_display, version)
    if for_submission:
        if is_fake(name, publisher, publisher_display):
            raise Refused("a package for submission needs the real identity from Partner Center, not the local-test one")
        if test_alias is not None:
            raise Refused("a package for submission has no test alias")
    values = {
        "IDENTITY_NAME": name,
        "PUBLISHER": publisher,
        "PUBLISHER_DISPLAY_NAME": publisher_display,
        "VERSION": version,
        "VIRTUALIZATION": virtualization_block(variant),
        "CAPABILITIES": capabilities_block(variant),
        "EXTRA_EXTENSIONS": alias_block(test_alias),
    }
    attribute_fields = {"IDENTITY_NAME", "PUBLISHER", "VERSION"}

    def fill(found: re.Match[str]) -> str:
        key = found.group(1)
        if key not in values:
            raise Refused(f"the template has an unknown field {{{{{key}}}}}")
        value = values[key]
        if key in attribute_fields:
            # Inside an attribute's quotes already: escape the quote-significant text only.
            return escape(value, {'"': "&quot;", "'": "&apos;"})
        return escape(value) if key == "PUBLISHER_DISPLAY_NAME" else value

    text = FIELD_RE.sub(fill, template)
    if FIELD_RE.search(text):
        raise Refused("a template field was left unfilled")
    return text


def startup_task_id() -> str:
    """`TASK_ID` in crates/desktop/src/startup.rs: the task the app asks Windows for."""
    found = re.search(r'pub const TASK_ID: &str = "([^"]+)";', STARTUP_RS.read_text(encoding="utf-8"))
    if not found:
        raise Refused("TASK_ID not found in crates/desktop/src/startup.rs")
    return found.group(1)


def stage(
    exe: Path,
    out: Path,
    *,
    manifest: str,
    assets: Path = ASSETS,
) -> list[str]:
    """Lays out the package folder `out` (emptied first) and returns its files."""
    if not exe.is_file():
        raise Refused(f"the exe {exe} is not there")
    if out.exists():
        # Only a folder this tool made (or an empty one) is emptied.
        if any(out.iterdir()) and not (out / "AppxManifest.xml").is_file():
            raise Refused(f"{out} is not empty and is not a package folder: not deleting it")
        shutil.rmtree(out)
    (out / "Assets").mkdir(parents=True)
    shutil.copyfile(exe, out / EXE_NAME)
    for png in sorted(assets.glob("*.png")):
        shutil.copyfile(png, out / "Assets" / png.name)
    (out / "AppxManifest.xml").write_text(manifest, encoding="utf-8", newline="\n")
    missing = [ref for ref in ASSET_REF_RE.findall(manifest) if not (out / ref.replace("\\", "/")).is_file()]
    if missing:
        raise Refused(f"the manifest names images that are not in Assets: {', '.join(missing)}")
    return sorted(path.relative_to(out).as_posix() for path in out.rglob("*") if path.is_file())


def updater_key() -> bytes:
    conf = json.loads(CONF.read_text(encoding="utf-8"))
    return conf["plugins"]["updater"]["pubkey"].encode("ascii")


def check_exe(data: bytes, *, key: bytes) -> list[str]:
    """What is wrong with this exe as the Store edition (empty: nothing)."""
    problems = []
    if STORE_MARK not in data:
        problems.append("not the Store edition: it was built without the `store` feature")
    if INSTALLER_MARK in data:
        problems.append("it carries the installer edition's name: the other edition's binary")
    if UPDATER_ENDPOINT in data:
        problems.append("it holds the updater's endpoint: the updater is not off")
    if key in data:
        problems.append("it holds the updater's public key: the config override was not applied")
    return problems


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    commands = parser.add_subparsers(dest="command", required=True)

    stage_cmd = commands.add_parser("stage")
    stage_cmd.add_argument("--exe", type=Path, required=True)
    stage_cmd.add_argument("--out", type=Path, required=True)
    stage_cmd.add_argument("--variant", choices=VARIANTS, default="full")
    stage_cmd.add_argument("--name", default=FAKE_NAME)
    stage_cmd.add_argument("--publisher", default=FAKE_PUBLISHER)
    stage_cmd.add_argument("--publisher-display", default=FAKE_PUBLISHER_DISPLAY)
    stage_cmd.add_argument("--version")
    stage_cmd.add_argument("--for-submission", action="store_true")
    stage_cmd.add_argument("--test-alias")

    identity_cmd = commands.add_parser("check-identity")
    identity_cmd.add_argument("--variant", choices=VARIANTS, default="full")
    identity_cmd.add_argument("--name", default=FAKE_NAME)
    identity_cmd.add_argument("--publisher", default=FAKE_PUBLISHER)
    identity_cmd.add_argument("--publisher-display", default=FAKE_PUBLISHER_DISPLAY)
    identity_cmd.add_argument("--for-submission", action="store_true")
    identity_cmd.add_argument("--test-alias")

    check_cmd = commands.add_parser("check-exe")
    check_cmd.add_argument("exe", type=Path)

    args = parser.parse_args(argv[1:])
    try:
        if args.command == "check-exe":
            problems = check_exe(args.exe.read_bytes(), key=updater_key())
            for problem in problems:
                print(f"error: {problem}", file=sys.stderr)
            return 1 if problems else 0
        version = getattr(args, "version", None) or store_version.store_version(store_version.app_version())
        manifest = render_manifest(
            TEMPLATE.read_text(encoding="utf-8"),
            name=args.name,
            publisher=args.publisher,
            publisher_display=args.publisher_display,
            version=version,
            variant=args.variant,
            test_alias=args.test_alias,
            for_submission=args.for_submission,
        )
        if f'TaskId="{startup_task_id()}"' not in manifest:
            raise Refused("the manifest's start-up task is not the one the app asks for")
        if args.command == "check-identity":
            return 0
        problems = check_exe(args.exe.read_bytes(), key=updater_key()) if args.exe.is_file() else []
        if problems:
            raise Refused("; ".join(problems))
        for path in stage(args.exe, args.out, manifest=manifest):
            print(path)
    except (Refused, store_version.BadVersion) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
