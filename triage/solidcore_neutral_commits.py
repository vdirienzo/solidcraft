#!/usr/bin/env python3
"""How many commits that touched the kernel's tree changed nothing that computes models?

The model cache is keyed by the digest of the whole `solidcore/` tree, so every commit that
touches it invalidates the cache of every saved design: the design is evaluated again on open.
This counts the commits that touched only files that cannot compute a model.
"""
import subprocess
import sys

ROOT = sys.argv[1] if len(sys.argv) > 1 else "."
SINCE = sys.argv[2] if len(sys.argv) > 2 else "2026-10-20"

MODEL_CRATES = ("solidcore", "solidcore-ops", "solidcore-record", "solidcore-step", "solidcore-convert")


def git(*args):
    out = subprocess.run(["git", "-C", ROOT, *args], capture_output=True, text=True)
    return out.stdout if out.returncode == 0 else ""


commits = [l.split()[0] for l in git("log", f"--since={SINCE}", "--format=%H", "--", "solidcore/").splitlines()]
neutral = []
for c in commits:
    paths = git("show", "--name-only", "--format=", c).splitlines()
    paths = [p for p in paths if p.startswith("solidcore/")]
    if not paths:
        continue
    computes = any(
        p.startswith(f"solidcore/crates/{crate}/src/") or p.endswith("Cargo.toml") or p.endswith("Cargo.lock") or "/.cargo/" in p
        for crate in MODEL_CRATES
        for p in paths
    )
    if not computes:
        neutral.append((c, paths))

print(f"commits that touched solidcore/ since {SINCE}: {len(commits)}")
print(f"of those, the ones that changed nothing that computes a model: {len(neutral)}")
for c, paths in neutral:
    subject = git("log", "-1", "--format=%s", c).strip()
    print(f"  {c[:9]} {subject}")
    for p in paths[:6]:
        print(f"      {p}")
    if len(paths) > 6:
        print(f"      ... and {len(paths) - 6} more")
