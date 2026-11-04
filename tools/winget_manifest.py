"""Write the winget manifests for the newest release, or check them (T-702).

The Windows Package Manager community repository (microsoft/winget-pkgs)
takes a package as a folder of YAML manifests per version. This script makes
that folder for the newest release in web/src/lib/site/releases.json (the
record `tools/release.py facts` writes from GitHub, so the hash is GitHub's
own SHA-256 of the published file) under packaging/winget/. Submitting it to
winget-pkgs is the owner's step (it needs the Microsoft CLA): see
packaging/winget/README.md.

    python tools/winget_manifest.py          # write the folder
    python tools/winget_manifest.py --check  # exit 1 if the folder is stale

Every scalar is written as a JSON string, which YAML reads as a
double-quoted string, so no value can change meaning in YAML. Standard
library only.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RELEASES = ROOT / "web" / "src" / "lib" / "site" / "releases.json"
OUT = ROOT / "packaging" / "winget" / "manifests"

PACKAGE_ID = "KBroneZ.TavernLedger"
# winget-pkgs keeps manifests at manifests/<first letter>/<publisher>/<name>/<version>/.
PACKAGE_DIR = ("k", "KBroneZ", "TavernLedger")
SCHEMA = "1.12.0"
REPO = "https://github.com/KBroneZ/tavern-ledger"
# What the installer writes in Apps & features (tauri.conf.json: productName,
# bundle.publisher; the NSIS uninstall key is named after the product).
DISPLAY_NAME = "Tavern Ledger"
PUBLISHER = "KBroneZ"

VERSION = re.compile(r"^\d+\.\d+\.\d+$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")

FAN_NOTICE = {
    "en-US": "Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment.",
    "es-ES": "Proyecto no oficial de fans. Sin afiliación con Blizzard Entertainment ni respaldo de esta.",
}

# Plain facts from the README and the download page; nothing the app does not do (rule 8).
# The fan notice is fixed text (docs/brand/voice.md); Spanish follows its glossary.
LOCALES = {
    "en-US": {
        "ShortDescription": "Free, open-source Hearthstone Battlegrounds tracker: every game, recaps, combat replays and an overlay.",
        "Description": (
            "Tavern Ledger keeps every Hearthstone Battlegrounds game you play, read from the game's own log: "
            "hero, place, health each round, the opponents you met and both boards at each combat. "
            "Each game has a recap and attack-by-attack combat replays, and the stats split Solo and Duos. "
            "An optional in-game overlay shows the last board you met from each opponent and the combat odds "
            "from the project's own simulator.\n"
            "By default it also reads two values the game shows you, the lobby's tribes and your own rating, "
            "read-only from the game's memory; they stay on your PC and you can turn that off in Settings. "
            "This goes against the letter of Blizzard's rules, and the account at risk is yours. "
            "It never changes the game and never plays for you. Your games stay on your PC; uploading to "
            "tavernledger.net is off until you sign in and turn it on.\n"
            "The app updates itself from the project's signed GitHub releases when you press Restart now.\n"
            + FAN_NOTICE["en-US"]
        ),
        "Tags": ["battlegrounds", "hearthstone", "game-tracker", "overlay", "combat-simulator", "open-source"],
    },
    "es-ES": {
        "ShortDescription": "Tracker gratuito y de código abierto de Campos de Batalla de Hearthstone: partidas, resúmenes, repeticiones y overlay.",
        "Description": (
            "Tavern Ledger guarda todas tus partidas de Campos de Batalla de Hearthstone, leídas del propio registro del juego: "
            "héroe, puesto, vida en cada ronda, los rivales con los que te cruzaste y los dos tableros de cada combate. "
            "Cada partida tiene su resumen y la repetición de cada combate ataque a ataque, y las estadísticas separan "
            "Solo y Dúos. Un overlay opcional en la partida muestra el último tablero que viste de cada rival y las "
            "probabilidades del combate según el simulador propio del proyecto.\n"
            "Por defecto también lee dos datos que el juego te muestra, las razas de la partida y tu propia puntuación, "
            "leídas de la memoria del juego sin escribir en ella; se quedan en tu PC y puedes desactivarlo en Ajustes. "
            "Esto va en contra de la letra de las normas de Blizzard, y la cuenta en riesgo es la tuya. "
            "Nunca modifica el juego ni juega por ti. Tus partidas se quedan en tu PC; la subida a tavernledger.net "
            "está desactivada hasta que inicias sesión y la activas.\n"
            "La aplicación se actualiza sola desde las versiones firmadas del proyecto en GitHub cuando pulsas Reiniciar ahora.\n"
            + FAN_NOTICE["es-ES"]
        ),
        "Tags": ["battlegrounds", "hearthstone", "campos-de-batalla", "tracker", "overlay", "codigo-abierto"],
    },
}
DEFAULT_LOCALE = "en-US"


class Refused(Exception):
    """The release record cannot be turned into manifests."""


def newest(releases: object) -> dict:
    """The newest release record, checked as release.ts checks it."""
    if not isinstance(releases, list) or not releases or not isinstance(releases[0], dict):
        raise Refused("releases.json: expected a non-empty list of releases")
    r = releases[0]
    version = r.get("version")
    if not isinstance(version, str) or not VERSION.match(version):
        raise Refused(f"releases.json: bad version {version!r}")
    if r.get("tag") != f"v{version}" or r.get("file") != f"TavernLedger_{version}_x64-setup.exe":
        raise Refused(f"releases.json: tag or file does not match version {version}")
    if not isinstance(r.get("sha256"), str) or not SHA256.match(r["sha256"]):
        raise Refused("releases.json: bad sha256")
    if not isinstance(r.get("date"), str) or not DATE.match(r["date"]):
        raise Refused("releases.json: bad date")
    return r


def _scalar(value: object) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False)
    raise TypeError(f"unsupported value {value!r}")


def _yaml(kind: str, fields: list[tuple[str, object]]) -> str:
    lines = [f"# yaml-language-server: $schema=https://aka.ms/winget-manifest.{kind}.{SCHEMA}.schema.json", ""]
    for key, value in fields:
        if value is None:
            continue
        if isinstance(value, list):
            lines.append(f"{key}:")
            for item in value:
                if isinstance(item, dict):
                    pairs = list(item.items())
                    lines.append(f"- {pairs[0][0]}: {_scalar(pairs[0][1])}")
                    lines.extend(f"  {k}: {_scalar(v)}" for k, v in pairs[1:])
                else:
                    lines.append(f"- {_scalar(item)}")
        else:
            lines.append(f"{key}: {_scalar(value)}")
    return "\n".join(lines) + "\n"


def manifests(release: dict) -> dict[str, str]:
    """File name to content for the release's manifest folder."""
    version, tag = release["version"], release["tag"]
    head = [("PackageIdentifier", PACKAGE_ID), ("PackageVersion", version)]
    files = {
        f"{PACKAGE_ID}.yaml": _yaml(
            "version",
            head + [("DefaultLocale", DEFAULT_LOCALE), ("ManifestType", "version"), ("ManifestVersion", SCHEMA)],
        ),
        f"{PACKAGE_ID}.installer.yaml": _yaml(
            "installer",
            head
            + [
                ("InstallerType", "nullsoft"),
                ("Scope", "user"),
                # Installing over an older version keeps the games and settings (D-170).
                ("UpgradeBehavior", "install"),
                # The app updates itself; `winget upgrade --all` leaves it alone.
                ("RequireExplicitUpgrade", True),
                ("ReleaseDate", release["date"]),
                ("AppsAndFeaturesEntries", [{"DisplayName": DISPLAY_NAME, "Publisher": PUBLISHER, "DisplayVersion": version, "ProductCode": DISPLAY_NAME}]),
                (
                    "Installers",
                    [
                        {
                            "Architecture": "x64",
                            "InstallerUrl": f"{REPO}/releases/download/{tag}/{release['file']}",
                            "InstallerSha256": release["sha256"].upper(),
                        }
                    ],
                ),
                ("ManifestType", "installer"),
                ("ManifestVersion", SCHEMA),
            ],
        ),
    }
    for locale, text in LOCALES.items():
        default = locale == DEFAULT_LOCALE
        fields = head + [
            ("PackageLocale", locale),
            ("Publisher", PUBLISHER),
            ("PublisherUrl", "https://github.com/KBroneZ"),
            ("PublisherSupportUrl", f"{REPO}/issues"),
            ("PrivacyUrl", "https://tavernledger.net/privacy/" if default else "https://tavernledger.net/es/privacy/"),
            ("PackageName", DISPLAY_NAME),
            ("PackageUrl", "https://tavernledger.net/" if default else "https://tavernledger.net/es/"),
            ("License", "AGPL-3.0-only"),
            ("LicenseUrl", f"{REPO}/blob/{tag}/LICENSE"),
            ("ShortDescription", text["ShortDescription"]),
            ("Description", text["Description"]),
            ("Moniker", "tavernledger" if default else None),
            ("Tags", text["Tags"]),
            ("ReleaseNotesUrl", f"{REPO}/releases/tag/{tag}"),
            ("ManifestType", "defaultLocale" if default else "locale"),
            ("ManifestVersion", SCHEMA),
        ]
        files[f"{PACKAGE_ID}.locale.{locale}.yaml"] = _yaml("defaultLocale" if default else "locale", fields)
    return files


def folder(out: Path, version: str) -> Path:
    return out.joinpath(*PACKAGE_DIR, version)


def stale(out: Path, release: dict) -> list[str]:
    """What differs between `out` and the manifests for `release`; empty when current."""
    problems = []
    expected = manifests(release)
    base = out.joinpath(*PACKAGE_DIR)
    if base.exists():
        for other in sorted(p.name for p in base.iterdir() if p.name != release["version"]):
            problems.append(f"{other}: only the newest release's folder is kept")
    target = folder(out, release["version"])
    present = {p.name for p in target.iterdir()} if target.is_dir() else set()
    for name, content in expected.items():
        path = target / name
        if not path.is_file():
            problems.append(f"{name}: missing")
        # read_text reads CRLF as LF, so a Windows checkout compares the same.
        elif path.read_text(encoding="utf-8") != content:
            problems.append(f"{name}: differs from the generator's output")
    problems.extend(f"{name}: not made by the generator" for name in sorted(present - set(expected)))
    return problems


def write(out: Path, release: dict) -> Path:
    base = out.joinpath(*PACKAGE_DIR)
    if base.exists():
        for old in base.iterdir():
            if old.name != release["version"] and old.is_dir():
                for f in old.iterdir():
                    f.unlink()
                old.rmdir()
    target = folder(out, release["version"])
    target.mkdir(parents=True, exist_ok=True)
    for name, content in manifests(release).items():
        (target / name).write_text(content, encoding="utf-8", newline="\n")
    return target


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="fail if packaging/winget is not the generator's output")
    args = parser.parse_args(argv)
    try:
        release = newest(json.loads(RELEASES.read_text(encoding="utf-8")))
    except (OSError, ValueError, Refused) as error:
        print(f"winget_manifest: {error}", file=sys.stderr)
        return 2
    if args.check:
        problems = stale(OUT, release)
        for line in problems:
            print(f"winget_manifest: {line}", file=sys.stderr)
        if problems:
            print("winget_manifest: run `python tools/winget_manifest.py` and commit the result", file=sys.stderr)
            return 1
        print(f"winget_manifest: packaging/winget is current for {release['version']}")
        return 0
    target = write(OUT, release)
    print(f"winget_manifest: wrote {target.relative_to(ROOT).as_posix()}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
