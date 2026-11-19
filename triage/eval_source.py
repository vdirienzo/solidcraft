#!/usr/bin/env python3
"""The digest crates/io/build.rs computes (SOLIDCRAFT_EVALUATION_SOURCE), for measuring.

Same algorithm as crates/io/build.rs at the given commit:
  - the tree id of every evaluation crate and every evaluation tree
  - the vendor entries (except the ones outside)
  - the locked dependencies (the workspace's own packages left out)
  - the workspace settings (the version left out)
"""
import hashlib
import subprocess
import sys

EVALUATION_CRATES = ["color", "geom", "profile", "smlib", "sketch", "kernel", "render", "derive", "develop", "doc", "format", "viewport"]
EVALUATION_FILES = ["crates/io/build.rs", "crates/io/src/design/native_cache.rs", ".cargo/config.toml"]
EVALUATION_TREES = ["solidcore"]
VENDOR_OUTSIDE = ["README.md", "egui-wgpu"]


def git(root, *args):
    out = subprocess.run(["git", "-C", root, *args], capture_output=True, text=True)
    if out.returncode != 0:
        return None
    return out.stdout


def locked_dependencies(lock):
    blocks = lock.split("\n[[package]]")
    kept = [b for b in blocks if any(l.startswith("source = ") for l in b.splitlines())]
    return "\n[[package]]".join(kept)


def workspace_settings(manifest):
    section = ""
    kept = []
    for line in manifest.splitlines():
        if line.startswith("["):
            section = line.strip()
        if not (section == "[workspace.package]" and line.lstrip().startswith("version")):
            kept.append(line)
    return "\n".join(kept)


def digest(root, rev="HEAD", crates=None, trees=None, files=None):
    crates = EVALUATION_CRATES if crates is None else crates
    trees = EVALUATION_TREES if trees is None else trees
    files = EVALUATION_FILES if files is None else files
    paths = [f"crates/{c}" for c in crates] + list(trees) + list(files)
    h = hashlib.sha256()
    for path in paths:
        one = git(root, "rev-parse", f"{rev}:./{path}")
        if one is None or not one.strip():
            continue  # a path that is not in this revision (a crate that did not exist yet)
        h.update(f"{path} {one.strip()}\n".encode())
    vendor = git(root, "ls-tree", rev, "./vendor/")
    if vendor:
        for entry in vendor.splitlines():
            name = entry.rsplit(["\t", "/"][0] if False else "\t", 1)[-1].rsplit("/", 1)[-1]
            if name not in VENDOR_OUTSIDE:
                h.update(f"vendor {entry}\n".encode())
    lock = git(root, "show", f"{rev}:./Cargo.lock")
    if lock:
        h.update(locked_dependencies(lock).encode())
    manifest = git(root, "show", f"{rev}:./Cargo.toml")
    if manifest:
        h.update(workspace_settings(manifest).encode())
    return h.hexdigest()


if __name__ == "__main__":
    root = sys.argv[1] if len(sys.argv) > 1 else "."
    rev = sys.argv[2] if len(sys.argv) > 2 else "HEAD"
    print(digest(root, rev))
