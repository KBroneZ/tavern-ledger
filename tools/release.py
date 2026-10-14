"""Release helpers for the Windows installer and its updater files (T-105b, T-569).

The release workflow (.github/workflows/release.yml) and the daily release
check (.github/workflows/release-check.yml) run these; a person runs `facts`
once a release is published. Standard library only. Nothing here holds or
reads a private key: signing is `tauri signer sign` in the workflow's
protected job, and checking a signature is `cargo run -p release-check`.

Usage:
    python tools/release.py prepare --ref REF [--dry-run]
        Checks that the tag, tauri.conf.json and the desktop crate agree on
        the version and that the updater public key is set; prints
        `version=...` and `file=...` lines for $GITHUB_OUTPUT.
    python tools/release.py trusted-key TAG
        Prints the public key that apps already installed trust, which must
        verify TAG's signatures: the key in the previous `v*` tag's
        tauri.conf.json, or TAG's own key for the first self-updating
        release. A release that changes the key is signed with the old one.
    python tools/release.py sums DIR FILE...
        Writes DIR/SHA256SUMS for the files (in DIR), `sha256sum` style.
    python tools/release.py latest-json VERSION SIG OUT [--notes TEXT]
        Writes the updater's latest.json for VERSION, whose installer
        signature (the .sig file's text) is in SIG.
    python tools/release.py minisig SIG OUT
        Writes the plain minisign form of a .sig file (for `minisign -V`).
    python tools/release.py facts TAG
        Reads the published release TAG from GitHub and records its files'
        sizes and SHA-256 in web/src/lib/site/releases.json (newest first).
    python tools/release.py check
        For every release in releases.json, the live files must still be the
        recorded ones. Exit 1 on any difference.

Exit codes: 0 = done, 1 = a check failed (said why), 2 = wrong use.
"""

from __future__ import annotations

import argparse
import base64
import datetime as dt
import hashlib
import json
import os
import re
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any, Callable

ROOT = Path(__file__).resolve().parent.parent
CONF = ROOT / "crates" / "desktop" / "tauri.conf.json"
CRATE = ROOT / "crates" / "desktop" / "Cargo.toml"
FACTS = ROOT / "web" / "src" / "lib" / "site" / "releases.json"
REPO = "KBroneZ/tavern-ledger"
API = f"https://api.github.com/repos/{REPO}/releases/tags/"
DOWNLOADS = f"https://github.com/{REPO}/releases/download/"
SUMS = "SHA256SUMS"
TIMEOUT = 30
USER_AGENT = "tavern-ledger-release-tools"
VERSION_RE = re.compile(r"^\d+\.\d+\.\d+$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
# Assets a release of ours may carry; anything else is refused by `facts`.
ASSET_RE = re.compile(r"^(TavernLedger_\d+\.\d+\.\d+_x64-setup\.exe(\.sig)?|latest\.json|SHA256SUMS(\.sig|\.minisig)?)$")

Fetch = Callable[[str], Any]


class Refused(Exception):
    """A check failed; the message says which and why."""


def installer_name(version: str) -> str:
    return f"TavernLedger_{version}_x64-setup.exe"


def installer_url(version: str) -> str:
    return f"{DOWNLOADS}v{version}/{installer_name(version)}"


# --- prepare ---------------------------------------------------------------


def is_public_key(key: object) -> bool:
    """Whether `key` is a minisign public key as `tauri signer generate`
    writes it: base64 of an `untrusted comment:` line and a base64 line of
    42 bytes ("Ed", the 8-byte key id, the 32-byte Ed25519 key)."""
    if not isinstance(key, str):
        return False
    try:
        text = base64.b64decode(key, validate=True).decode("utf-8")
        lines = text.strip().splitlines()
        if len(lines) != 2 or not lines[0].startswith("untrusted comment:"):
            return False
        raw = base64.b64decode(lines[1].strip(), validate=True)
    except (ValueError, UnicodeDecodeError):
        return False
    return len(raw) == 42 and raw[:2] == b"Ed"


def crate_version(cargo_toml: str) -> str | None:
    """The `version` of the `[package]` table (the first `version =` line)."""
    in_package = False
    for line in cargo_toml.splitlines():
        stripped = line.strip()
        if stripped.startswith("["):
            in_package = stripped == "[package]"
        elif in_package:
            m = re.match(r'^version\s*=\s*"([^"]+)"', stripped)
            if m:
                return m.group(1)
    return None


def prepare(ref: str, dry_run: bool, conf_text: str, crate_text: str) -> dict[str, str]:
    conf = json.loads(conf_text)
    version = conf.get("version")
    if not isinstance(version, str) or not VERSION_RE.match(version):
        raise Refused(f"tauri.conf.json version {version!r} is not X.Y.Z")
    if crate_version(crate_text) != version:
        raise Refused(f"crates/desktop/Cargo.toml version is {crate_version(crate_text)!r}, tauri.conf.json says {version}")
    if not dry_run and ref != f"refs/tags/v{version}":
        raise Refused(f"the tag {ref!r} is not v{version}, the version in tauri.conf.json")
    key = conf.get("plugins", {}).get("updater", {}).get("pubkey")
    if not is_public_key(key) and not dry_run:
        raise Refused("plugins.updater.pubkey in tauri.conf.json is not the release public key (still the placeholder?)")
    return {"version": version, "file": installer_name(version)}


def updater_key(conf_text: str | None) -> str | None:
    """The config's updater public key when it is a real one."""
    if conf_text is None:
        return None
    try:
        key = json.loads(conf_text).get("plugins", {}).get("updater", {}).get("pubkey")
    except (json.JSONDecodeError, AttributeError):
        return None
    return key if is_public_key(key) else None


Git = Callable[[list[str]], str | None]


def run_git(args: list[str]) -> str | None:
    """git's output, or None when it fails (no such tag or file)."""
    res = subprocess.run(["git", *args], capture_output=True, text=True, cwd=ROOT, check=False)
    return res.stdout if res.returncode == 0 else None


def trusted_key(tag: str, git: Git) -> tuple[str, str]:
    """The key that must verify `tag`'s signatures, and where it comes from."""
    path = "crates/desktop/tauri.conf.json"
    own = updater_key(git(["show", f"{tag}:{path}"]))
    if own is None:
        raise Refused(f"{tag}: tauri.conf.json has no release public key")
    # The newest earlier release tag (vX.Y.Z only) whose config has a real
    # key: a stray tag or an old release without the updater is skipped.
    earlier = git(["tag", "--merged", f"{tag}^", "--list", "v*", "--sort=-v:refname"]) or ""
    previous, old = "", None
    for name in earlier.split():
        if VERSION_RE.match(name[1:]) and name.startswith("v"):
            old = updater_key(git(["show", f"{name}:{path}"]))
            if old is not None:
                previous = name
                break
    if old is None:
        return own, f"{tag} is the first release with an updater key: checked against its own key"
    if old != own:
        return old, f"{tag} changes the updater key: checked against {previous}'s key, which installed apps trust"
    return own, f"checked against the key of {tag} and {previous}"


# --- files -----------------------------------------------------------------


def sha256_of(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def write_sums(folder: Path, names: list[str]) -> str:
    text = "".join(f"{sha256_of(folder / name)}  {name}\n" for name in sorted(names))
    (folder / SUMS).write_text(text, encoding="utf-8", newline="\n")
    return text


def latest_json(version: str, signature: str, notes: str, now: dt.datetime) -> str:
    if not VERSION_RE.match(version):
        raise Refused(f"version {version!r} is not X.Y.Z")
    signature = signature.strip()
    base64.b64decode(signature, validate=True)
    entry = {"signature": signature, "url": installer_url(version)}
    doc = {
        "version": version,
        "notes": notes,
        "pub_date": now.astimezone(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        # The NSIS build looks for its own target first, then the plain one.
        "platforms": {"windows-x86_64-nsis": entry, "windows-x86_64": entry},
    }
    return json.dumps(doc, indent=2) + "\n"


def minisig(signature: str) -> str:
    text = base64.b64decode(signature.strip(), validate=True).decode("utf-8")
    if not text.startswith("untrusted comment:") or "\ntrusted comment:" not in text:
        raise Refused("the .sig file is not a minisign signature")
    return text if text.endswith("\n") else text + "\n"


# --- facts and the daily check ---------------------------------------------


def fetch_json(url: str) -> Any:
    headers = {"Accept": "application/vnd.github+json", "User-Agent": USER_AGENT}
    token = os.environ.get("GITHUB_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    req = urllib.request.Request(url, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=TIMEOUT) as res:  # noqa: S310 (fixed https host)
            return json.loads(res.read(5_000_000))
    except urllib.error.HTTPError as e:
        raise Refused(f"{url}: HTTP {e.code}") from e
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError) as e:
        raise Refused(f"{url}: {e}") from e


def assets_of(release: Any, tag: str) -> dict[str, dict[str, Any]]:
    """The release's files with their sizes and SHA-256, checked for shape."""
    if not isinstance(release, dict) or release.get("tag_name") != tag:
        raise Refused(f"{tag}: not the release asked for")
    if release.get("draft"):
        raise Refused(f"{tag}: still a draft")
    out: dict[str, dict[str, Any]] = {}
    for asset in release.get("assets") or []:
        name = asset.get("name")
        digest = asset.get("digest") or ""
        size = asset.get("size")
        if not isinstance(name, str) or not ASSET_RE.match(name):
            raise Refused(f"{tag}: unexpected file {name!r}")
        if asset.get("state") != "uploaded" or not isinstance(size, int) or size <= 0:
            raise Refused(f"{tag}: {name} is not a complete upload")
        sha = digest.removeprefix("sha256:")
        if not digest.startswith("sha256:") or not SHA256_RE.match(sha):
            raise Refused(f"{tag}: {name} has no SHA-256 digest from GitHub")
        out[name] = {"bytes": size, "sha256": sha}
    return out


def facts_of(release: Any, tag: str, sums_text: str | None) -> dict[str, Any]:
    if not tag.startswith("v") or not VERSION_RE.match(tag[1:]):
        raise Refused(f"{tag!r} is not vX.Y.Z")
    version = tag[1:]
    assets = assets_of(release, tag)
    file = installer_name(version)
    if file not in assets:
        raise Refused(f"{tag}: no {file}")
    if sums_text is not None:
        pairs = (line.split("  ", 1) for line in sums_text.splitlines() if "  " in line)
        listed = {name: sha for sha, name in pairs}
        if listed.get(file) != assets[file]["sha256"]:
            raise Refused(f"{tag}: {SUMS} does not list the installer's SHA-256")
    published = release.get("published_at") or ""
    if not re.match(r"^\d{4}-\d{2}-\d{2}T", published):
        raise Refused(f"{tag}: no publication date")
    return {
        "version": version,
        "tag": tag,
        "file": file,
        "bytes": assets[file]["bytes"],
        "sha256": assets[file]["sha256"],
        "date": published[:10],
        "sums": SUMS in assets,
        "assets": dict(sorted(assets.items())),
    }


def fetch_text(url: str, limit: int = 100_000) -> str:
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    try:
        with urllib.request.urlopen(req, timeout=TIMEOUT) as res:  # noqa: S310 (fixed https host)
            return res.read(limit).decode("utf-8")
    except (urllib.error.URLError, TimeoutError, UnicodeDecodeError) as e:
        raise Refused(f"{url}: {e}") from e


def record(tag: str, fetch: Fetch, fetch_sums: Callable[[str], str], known: list[dict[str, Any]]) -> list[dict[str, Any]]:
    release = fetch(API + tag)
    names = {a.get("name") for a in (release.get("assets") or []) if isinstance(a, dict)}
    sums = fetch_sums(f"{DOWNLOADS}{tag}/{SUMS}") if SUMS in names else None
    entry = facts_of(release, tag, sums)
    rest = [r for r in known if r.get("tag") != tag]
    return sorted([entry, *rest], key=lambda r: [int(n) for n in r["version"].split(".")], reverse=True)


def check_all(known: list[dict[str, Any]], fetch: Fetch) -> list[str]:
    """What differs between the recorded releases and the live ones."""
    if not known:
        return ["releases.json lists no release"]
    problems = []
    for entry in known:
        tag = entry.get("tag", "?")
        try:
            live = assets_of(fetch(API + tag), tag)
        except Refused as e:
            problems.append(str(e))
            continue
        for name, want in entry.get("assets", {}).items():
            got = live.get(name)
            if got != want:
                problems.append(f"{tag}: {name} is now {got}, recorded {want}")
        for name in sorted(set(live) - set(entry.get("assets", {}))):
            problems.append(f"{tag}: {name} was added after it was recorded")
    return problems


def read_facts() -> list[dict[str, Any]]:
    return json.loads(FACTS.read_text(encoding="utf-8")) if FACTS.exists() else []


def write_facts(entries: list[dict[str, Any]]) -> None:
    FACTS.write_text(json.dumps(entries, indent=2) + "\n", encoding="utf-8", newline="\n")


# --- command line ------------------------------------------------------------


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("prepare")
    p.add_argument("--ref", required=True)
    p.add_argument("--dry-run", action="store_true")
    p = sub.add_parser("trusted-key")
    p.add_argument("tag")
    p = sub.add_parser("sums")
    p.add_argument("dir", type=Path)
    p.add_argument("files", nargs="+")
    p = sub.add_parser("latest-json")
    p.add_argument("version")
    p.add_argument("sig", type=Path)
    p.add_argument("out", type=Path)
    p.add_argument("--notes", default="")
    p = sub.add_parser("minisig")
    p.add_argument("sig", type=Path)
    p.add_argument("out", type=Path)
    p = sub.add_parser("facts")
    p.add_argument("tag")
    sub.add_parser("check")
    args = parser.parse_args(argv)
    try:
        if args.cmd == "prepare":
            out = prepare(args.ref, args.dry_run, CONF.read_text(encoding="utf-8"), CRATE.read_text(encoding="utf-8"))
            if args.dry_run and not is_public_key(json.loads(CONF.read_text(encoding="utf-8"))["plugins"]["updater"]["pubkey"]):
                print("::warning::dry run: the updater public key is still the placeholder", file=sys.stderr)
            print("\n".join(f"{k}={v}" for k, v in out.items()))
        elif args.cmd == "trusted-key":
            key, note = trusted_key(args.tag, run_git)
            print(f"::notice::{note}", file=sys.stderr)
            print(key)
        elif args.cmd == "sums":
            print(write_sums(args.dir, args.files), end="")
        elif args.cmd == "latest-json":
            text = latest_json(args.version, args.sig.read_text(encoding="utf-8"), args.notes, dt.datetime.now(dt.timezone.utc))
            args.out.write_text(text, encoding="utf-8", newline="\n")
        elif args.cmd == "minisig":
            args.out.write_text(minisig(args.sig.read_text(encoding="utf-8")), encoding="utf-8", newline="\n")
        elif args.cmd == "facts":
            entries = record(args.tag, fetch_json, fetch_text, read_facts())
            write_facts(entries)
            print(json.dumps(entries[0], indent=2))
        elif args.cmd == "check":
            problems = check_all(read_facts(), fetch_json)
            for problem in problems:
                print(f"::error::{problem}")
            if problems:
                return 1
            print("Every recorded release file is unchanged.")
    except Refused as e:
        print(f"::error::{e}", file=sys.stderr)
        return 1
    except (OSError, ValueError) as e:
        print(f"::error::{e}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
