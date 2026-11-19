#!/usr/bin/env python3
"""Releases where the model cache is invalidated although nothing that computes a model changed.

For every pair of consecutive releases it compares:
  * the key as it is today (crates/io/build.rs): the app's model crates as whole trees, the
    WHOLE `solidcore/` tree, the vendor tree, the locked dependencies and the workspace settings;
  * the key of the sources that compute a model: the app's model crates and the kernel's crates
    the app's kernel is built from, file by file, without their test files.
A pair where the first changed and the second did not is a release that made every saved design
rebuild its whole timeline for nothing.
"""
import hashlib
import subprocess
import sys

sys.path.insert(0, "/tmp/opencode")
from eval_source import digest, EVALUATION_CRATES, EVALUATION_FILES, EVALUATION_TREES  # noqa: E402

ROOT = sys.argv[1] if len(sys.argv) > 1 else "."
SINCE = sys.argv[2] if len(sys.argv) > 2 else "2026-09-01"

APP_CRATES = EVALUATION_CRATES
KERNEL_CRATES = ["solidcore", "solidcore-ops", "solidcore-record", "solidcore-step", "solidcore-convert"]
KERNEL_FILES = ["solidcore/Cargo.toml", "solidcore/Cargo.lock", "solidcore/.cargo/config.toml"]
TEST_FILES = ("tests.rs", "test.rs")


def git(*args):
    out = subprocess.run(["git", "-C", ROOT, *args], capture_output=True, text=True)
    return out.stdout if out.returncode == 0 else ""


def is_test_file(path):
    name = path.rsplit("/", 1)[-1]
    return name.endswith("_tests.rs") or name in TEST_FILES or "/tests/" in path or name.startswith("test_")


def sources(rev, crates, kernel=False):
    """The source files of the crates, without their test files."""
    out = []
    for c in crates:
        base = f"solidcore/crates/{c}" if kernel else f"crates/{c}"
        listing = git("ls-tree", "-r", rev, f"{base}/src/")
        for line in listing.splitlines():
            parts = line.split("\t")
            if len(parts) != 2:
                continue
            path = parts[1]
            if is_test_file(path):
                continue
            out.append((path, parts[0].split()[2]))
    return out


def sources_key(rev, crates, kernel_files):
    h = hashlib.sha256()
    for path, blob in sources(rev, APP_CRATES) + sources(rev, KERNEL_CRATES, kernel=True):
        h.update(f"{path} {blob}\n".encode())
    for f in kernel_files:
        one = git("rev-parse", f"{rev}:./{f}")
        if one.strip():
            h.update(f"{f} {one.strip()}\n".encode())
    return h.hexdigest()


releases = []
for line in git("log", f"--since={SINCE}", "--format=%H %s", "--grep=^Release ").splitlines():
    sha, subject = line.split(" ", 1)
    releases.append((sha, subject))
releases.reverse()


def key(rev, crates, trees, files):
    return digest(ROOT, rev, crates=crates, trees=trees, files=files)


waste = []
for (old, old_subject), (new, new_subject) in zip(releases, releases[1:]):
    today_old = key(old, APP_CRATES, EVALUATION_TREES, EVALUATION_FILES)
    today_new = key(new, APP_CRATES, EVALUATION_TREES, EVALUATION_FILES)
    if today_old == today_new:
        continue
    model_old = sources_key(old, APP_CRATES, KERNEL_FILES)
    model_new = sources_key(new, APP_CRATES, KERNEL_FILES)
    if model_old == model_new:
        waste.append((old_subject, new_subject))
        print(f"WASTED: {old_subject} -> {new_subject}")
    else:
        print(f"real:   {old_subject} -> {new_subject}")

print(f"\nreleases since {SINCE}: {len(releases)}")
print(f"releases that invalidated every saved design although no model source changed: {len(waste)}")
