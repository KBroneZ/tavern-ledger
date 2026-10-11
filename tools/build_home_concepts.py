"""Builds the one-file review page of the home page concepts (T-525) for publishing as an Artifact.

The concepts in docs/brand/home/ are separate pages that share shared/tavern.css and
shared/tavern.js, and the review page (index.html) shows each one in frames. A published
Artifact is a single page, so this script:

- inlines the shared CSS and JS into every concept and into the review page,
- puts each concept into its frame as `srcdoc` instead of `src`,
- marks every concept `data-portraits="silhouette"`: the Artifact's security policy does not
  allow HearthstoneJSON's image host, so the pages draw our own silhouettes instead of
  requesting the portraits,
- shows the review page's note that says so (`data-artifact-only`) and drops what only works
  from the folder (`data-local-only`, the "Open the page" links),
- keeps only the head's title, links and styles and the body's content, because the Artifact
  wraps the page in its own document.

Usage: python tools/build_home_concepts.py [OUTPUT]   (default .local/home-concepts-review.html)
Reads only the files in docs/brand/home/; writes only OUTPUT.
"""

from __future__ import annotations

import html
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HOME = ROOT / "docs" / "brand" / "home"
DEFAULT_OUT = ROOT / ".local" / "home-concepts-review.html"

CSS_LINK = '<link rel="stylesheet" href="shared/tavern.css">'
JS_TAG = '<script src="shared/tavern.js"></script>'
CONCEPT_NAME = re.compile(r"^[a-z]-[a-z0-9-]+\.html$")
IFRAME_SRC = re.compile(r'<iframe src="([^"]+)"')
LOCAL_ONLY = re.compile(r'<a class="open-full" data-local-only href="[^"]*">[^<]*</a>')


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def inline_shared(page: str, css: str, js: str) -> str:
    """Replaces the shared stylesheet and script tags with their contents."""
    if CSS_LINK not in page:
        raise ValueError("page does not link shared/tavern.css")
    page = page.replace(CSS_LINK, f"<style>\n{css}</style>")
    if JS_TAG in page:
        page = page.replace(JS_TAG, f"<script>\n{js}</script>")
    return page


def concept_for_artifact(name: str, css: str, js: str) -> str:
    """One concept page, self-contained, drawing silhouettes instead of loading portraits."""
    if not CONCEPT_NAME.match(name):
        raise ValueError(f"not a concept page: {name!r}")
    page = inline_shared(read(HOME / name), css, js)
    page, count = re.subn(r"<html lang=\"es\"", '<html lang="es" data-portraits="silhouette"', page, count=1)
    if count != 1:
        raise ValueError(f"{name}: no <html lang=\"es\"> to mark")
    return page


def strip_document(page: str) -> str:
    """Keeps the head's content and the body's content, without the document tags."""
    head = re.search(r"<head>(.*?)</head>", page, re.S)
    body = re.search(r"<body>(.*?)</body>", page, re.S)
    if not head or not body:
        raise ValueError("review page has no <head> or <body>")
    keep = [line for line in head.group(1).splitlines() if not line.strip().startswith("<meta")]
    return "\n".join(keep).strip() + "\n" + body.group(1).strip() + "\n"


def build() -> str:
    css = read(HOME / "shared" / "tavern.css")
    js = read(HOME / "shared" / "tavern.js")
    page = inline_shared(read(HOME / "index.html"), css, js)

    cache: dict[str, str] = {}

    def to_srcdoc(match: re.Match[str]) -> str:
        name = match.group(1)
        if name not in cache:
            cache[name] = concept_for_artifact(name, css, js)
        return f'<iframe srcdoc="{html.escape(cache[name], quote=True)}"'

    page = IFRAME_SRC.sub(to_srcdoc, page)
    if not cache:
        raise ValueError("review page has no concept frames")
    page = LOCAL_ONLY.sub("", page)
    page = page.replace(" data-artifact-only hidden>", " data-artifact-only>")
    return strip_document(page)


def main(argv: list[str]) -> int:
    out = Path(argv[1]) if len(argv) > 1 else DEFAULT_OUT
    text = build()
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(text, encoding="utf-8", newline="\n")
    print(f"wrote {out} ({len(text.encode('utf-8')) // 1024} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
