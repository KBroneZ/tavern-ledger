"""Fail a pull request that changes the updater public key without saying so.

`plugins.updater.pubkey` in `crates/desktop/tauri.conf.json` is the key every
installed app trusts for updates. Changing it is a key rotation: releases
signed with the old key stop updating apps that have the new one, and the
other way round. It must be deliberate, so a change to it passes only when
the pull request body or one of its commit messages has a line saying
`updater-key-change: yes` (any letter case).

Reads the config at `--base` (a commit) and in the working tree, the pull
request body from the environment variable named by `--body-env` (an
environment variable, not an argument or a shell string, so nothing in the
body is ever run) and the commit messages in `--base..HEAD` from git. It
needs no token or secret, so it also runs on pull requests from forks.
Standard library only; it changes nothing.

Usage:
    python tools/check_updater_key.py --base <commit> [--body-env PR_BODY]

Exit codes: 0 = key unchanged or the change is announced, 1 = changed
without the marker, 2 = the config could not be read.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CONFIG = "crates/desktop/tauri.conf.json"
MARKER = re.compile(r"^[ \t]*updater-key-change:[ \t]*yes[ \t]*$", re.IGNORECASE | re.MULTILINE)


def pubkey(config_text: str | None) -> str | None:
    """The updater public key in a config's text; None when there is none."""
    if config_text is None:
        return None
    updater = json.loads(config_text).get("plugins", {}).get("updater", {})
    key = updater.get("pubkey") if isinstance(updater, dict) else None
    return key if isinstance(key, str) else None


def announced(*texts: str) -> bool:
    """Whether any of the texts has the marker on a line of its own."""
    return any(MARKER.search(text) for text in texts)


def problem(before: str | None, after: str | None, texts: list[str]) -> str | None:
    """The reason to fail, or None."""
    if before == after or announced(*texts):
        return None
    return (
        "the updater public key (plugins.updater.pubkey in "
        f"{CONFIG}) changed. Apps already installed trust the old key; if this "
        "is a deliberate key rotation, put the line `updater-key-change: yes` "
        "in the pull request description or in a commit message and push again. "
        "Otherwise restore the key."
    )


def override_files(root: Path) -> list[str]:
    """Other Tauri config files that mention a pubkey (Tauri merges them in)."""
    folder = (root / CONFIG).parent
    found = []
    for path in sorted(folder.glob("[Tt]auri*")):
        if path.is_file() and path.name != Path(CONFIG).name and "pubkey" in path.read_text(
            encoding="utf-8", errors="replace"
        ):
            found.append(path.name)
    return found


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=ROOT, check=True, capture_output=True, text=True, encoding="utf-8"
    ).stdout


def config_at(base: str) -> str | None:
    """The config in commit `base`; None when it did not exist there."""
    git("rev-parse", "--verify", "--quiet", f"{base}^{{commit}}")
    try:
        return git("show", f"{base}:{CONFIG}")
    except subprocess.CalledProcessError:
        return None


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--base", required=True, help="the commit the pull request targets")
    parser.add_argument("--body-env", default="PR_BODY", help="environment variable with the PR body")
    args = parser.parse_args(argv)
    try:
        before = pubkey(config_at(args.base))
        after = pubkey((ROOT / CONFIG).read_text(encoding="utf-8"))
        messages = git("log", "--format=%B%x00", f"{args.base}..HEAD")
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"::error::cannot read the updater key: {error}")
        return 2
    extra = override_files(ROOT)
    if extra:
        print(f"::error::{', '.join(extra)} sets a pubkey outside {CONFIG}; this check cannot see it. Keep the key in {CONFIG}.")
        return 1
    texts = [os.environ.get(args.body_env, ""), *messages.split("\x00")]
    found = problem(before, after, texts)
    if found:
        print(f"::error::{found}")
        return 1
    print("updater key unchanged" if before == after else "updater key changed, and announced")
    return 0


if __name__ == "__main__":
    sys.exit(main())
