# #10116 — diagnosis of the `window_system` reset (part B)

**Status: no PR from this branch.** PR #10147 (cfaulkingham, "Preserve the saved window system
when changing graphics settings") already implements this fix upstream; this branch is a
duplicate and is kept only as the written diagnosis. No RED test was reached here.

## Root cause: the value is lost in `save_graphics()`, on the way out

`window_system` lives in **two** stores, and the UI writes one of them back over the other:

| store | written by | read by |
|---|---|---|
| `graphics.json` → `window_system` | `prefs::save_graphics()` (`crates/ui-egui/src/prefs.rs:2317`) | the **host, before the window opens**: `window_system_preference()` (`apps/solidcraft/src/main.rs:1367`), used at `main.rs:1038-1048` to pick X11 vs native Wayland |
| `app.ron` → `preferences.window_system` | `SolveApp::prefs()` → `"preferences": self.preferences` (`crates/ui-egui/src/lib.rs:1010`) | the **UI**: `load_prefs()` → `prefs::lenient()` → `self.preferences = p` (`crates/ui-egui/src/lib.rs:1089-1095`) |

The host honours `graphics.json` at startup. The UI, however, edits the **app.ron** copy, and
`save_graphics()` writes that copy back into `graphics.json`:

```rust
// crates/ui-egui/src/prefs.rs:2325-2327
graphics["msaa"] = json!(app.preferences.msaa);
graphics["window_system"] = json!(app.preferences.window_system);   // ← the stale app.ron value
graphics["adapter"] = json!(app.preferences.graphics_adapter);
```

`save_graphics()` runs whenever **any** of the three graphics settings changed (the OK/Apply path
at `prefs.rs:2463-2468`, and the Cancel/Restore-Defaults paths at `prefs.rs:1623-1625` and
`prefs.rs:1981-1983`). So changing only anti-aliasing rewrites `window_system` too, with the
value that came from `app.ron` — where it is `"auto"` unless the user changed it *in the UI*
(`Prefs::default()`, `prefs.rs:974`). A `graphics.json` hand-edited to `"wayland"` (the reporter's
workaround) is therefore overwritten with `"auto"` on the next save, and the stutter returns.

**The asymmetry that let it through:** `main.rs:1115-1118` already re-read the *adapter* from
`graphics.json` into `app.preferences.graphics_adapter` after `load_prefs()` — exactly the
reconciliation `window_system` needed. `window_system` was never re-read, so it kept the stale
app.ron value while the host was using the file's value.

## What #10147 does, and whether it is complete

#10147 fixes it on the **read** side, in the host: a new `restore_graphics_preferences()`
(`apps/solidcraft/src/main.rs`) re-reads `graphics.json` after `load_prefs()` and writes both
`graphics_adapter` and `window_system` back into `app.preferences`. With the in-memory value equal
to the file's value, `save_graphics()` writes back the same thing and the choice survives.

Coverage of the reported symptom — **complete**:

- the reset on any graphics change (its test drives the real `ui.prefs` save path and asserts
  `graphics.json` keeps `wayland` afterwards);
- stale app.ron (`app.preferences.window_system` = `auto`/`wayland`/`x11`) vs a fresh launch with
  no saved UI state;
- `wayland` / `x11` / `auto`, surrounding whitespace, missing key, non-string and unknown values
  (all but `wayland`/`x11` → `auto`, matching `window_system_override()` at `main.rs:1374-1380`);
- an environment override (`SOLIDCRAFT_WAYLAND=0`) affects the session without replacing the
  saved preference.

Residuals I noted but did **not** verify by running (so, not claims):

- the restore is inside `if !safe_mode` (`main.rs:1126`), while `services.graphics_path` points at
  the real settings directory in safe mode too (`main.rs:534`); a graphics save made from a
  safe-mode session would therefore still write `"auto"` into the user's real `graphics.json`.
  This behaviour is unchanged by #10147 (it is the same before and after), so it is not a
  regression — but it is the same bug class in a path nobody covers.

## What is still open, and is worth a separate PR of ours

**Sub-problem (A) — the adapter choice is ignored in silence — is NOT covered by #10147.**
#10147 is scoped to the preferences-persistence half; it says so itself ("the issue remains open
for its separate multi-GPU adapter-selection request").

There *is* a warning, but it cannot be seen where the reporter looked:

```rust
// apps/solidcraft/src/main.rs:1129-1134
if !app.preferences.graphics_adapter.is_empty() && !info.name.eq_ignore_ascii_case(app.preferences.graphics_adapter.as_str()) {
    eprintln!("SolidCraft graphics: preferred adapter {:?} was not used; using {:?} instead", ...);
}
```

- it goes to **stderr**, which is not captured into the crash journal (no stderr capture in
  `crates/engine/src/crash.rs`), so it never reaches the **session log** the reporter quoted
  (`Renderer: Vulkan NVIDIA GeForce RTX 5070 Ti`, written by `crash::set_environment()` at
  `main.rs:1152-1155`, read back by `crates/ui-egui/src/issue_report.rs:562`);
- it only compares the **saved preference** with the adapter in use, so a request made through
  `WGPU_ADAPTER_NAME` (no saved preference) produces **no warning at all** — the reporter's
  second case.

A separate PR could surface the mismatch where the user sees it (the session log via
`crash::set_environment()`, and a message in the UI). It needs **its own issue**: #10116 is
already used by #10147, and the repo rule is one issue per PR.

**Sub-problem (C) — the XWayland stutter on a multi-GPU box — is not reproducible here** (one GPU
on this machine) and is environmental, not a code defect. The reporter's own workaround (edit
`graphics.json` by hand) is what part B was about; #7414 is referenced upstream for the
"auto should prefer native Wayland" half.

## If this branch is ever revived

The RED test would go in `crates/ui-egui/src/prefs.rs`'s `mod tests` (the area's existing test
file — AGENTS.md forbids a per-bug test file), next to
`the_window_system_is_saved_for_the_next_start` (`prefs.rs:7020`), driving the same
`graphics.json` round-trip #10147's tests drive. Note that #10147's tests live in the host
(`apps/solidcraft/src/main.rs`), because that is where the restore happens.
