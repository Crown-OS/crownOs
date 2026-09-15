#!/usr/bin/env python3
"""Fail if a markdown link points at a file or heading that does not exist.

The documentation used to live in a separate repository and rotted there: links
kept pointing at pages whose headings had been renamed, and nothing noticed
because nothing checked. Now that the docs sit beside the code, this runs in CI
so that cannot happen twice.

Only in-repo links are checked -- relative paths, same-file anchors, and absolute
URLs into this repository (which per-crate READMEs must use, because they are
rendered standalone on crates.io where a relative link has nothing to resolve
against). External URLs are deliberately not fetched; checking those means
network flakiness failing an unrelated pull request.
"""

import pathlib
import re
import sys
import urllib.parse

ROOT = pathlib.Path(__file__).resolve().parent.parent
SELF = "https://github.com/Crown-OS/crownOs/blob/main/"


def anchors(path):
    """GitHub's heading -> fragment rule: lowercase, drop punctuation, spaces to dashes."""
    text = path.read_text(encoding="utf-8", errors="ignore")
    return {
        re.sub(r"[^\w\s-]", "", heading.lower()).strip().replace(" ", "-")
        for heading in re.findall(r"^#{1,6}\s+(.*)$", text, re.M)
    }


def main():
    problems, checked = [], 0
    files = [
        f
        for f in ROOT.rglob("*.md")
        if ".git/" not in str(f) and "/target/" not in str(f)
    ]

    for f in files:
        for _text, url in re.findall(r"\[([^\]]*)\]\(([^)\s]+)\)", f.read_text()):
            if url.startswith(SELF):
                target = ROOT / url[len(SELF) :]
            elif url.startswith("#"):
                target = pathlib.Path(str(f) + url)
            elif url.startswith(("http://", "https://", "mailto:")):
                continue
            else:
                target = f.parent / url

            checked += 1
            path, _, frag = str(target).partition("#")
            path = pathlib.Path(urllib.parse.unquote(path))
            rel = f.relative_to(ROOT)

            if not path.exists():
                problems.append(f"{rel}: no such file -- {url}")
            elif frag and path.suffix == ".md" and frag not in anchors(path):
                problems.append(f"{rel}: no heading '#{frag}' in {path.name} -- {url}")

    print(f"checked {checked} in-repo links across {len(files)} markdown files")
    for p in problems:
        print(f"  {p}", file=sys.stderr)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
