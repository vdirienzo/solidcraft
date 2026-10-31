# Diagnosis of #10138 (no fix here; PR #10183 covers it)

## Why the app starts in English when no language is saved

The interface language is one field of the preferences struct, and its default is the
literal `"en"`:

- `crates/ui-egui/src/prefs.rs:1080` — `language: String = "en".into(), 0;`
  (declared by the `preference_settings!` macro; `0` is `PAGE_GENERAL`).

The chain from that field to the strings on screen:

1. `SolveApp::new` sets `preferences: prefs::Prefs::default()` (`crates/ui-egui/src/lib.rs:800`),
   so with no saved settings the language is `"en"`.
2. A saved settings file is applied by `SolveApp::load_prefs` (`crates/ui-egui/src/lib.rs:1029`),
   which reads `v["preferences"]` through `prefs::lenient` (`crates/ui-egui/src/prefs.rs:3776`).
   `lenient` deserializes with `#[serde(default)]`, so a file that has no `language` key
   also yields the `"en"` default.
3. The language reaches the UI through `crate::i18n::scope(&self.preferences.language)`
   (`crates/ui-egui/src/lib.rs:2795`), which sets a thread-local `CURRENT` locale that
   `i18n::tr`/`i18n::tr_label` read.

Nothing in the tree reads the OS locale: `grep` for `LANG`, `LC_ALL`,
`GetUserDefaultLocaleName`, `navigator().language` and `sys_locale` over `crates/` and
`apps/` returns nothing. There is no place where a system locale is consulted, so the
only source of the interface language is the preference, and its default is English.

The preferences UI offers the registered catalogs and nothing else
(`crates/ui-egui/src/prefs/general.rs:12-18`, iterating `crate::i18n::LOCALES`), so there
is no "System" entry either.

## What PR #10183 changes

`gh pr view 10183` (author `dahakbuilds`, head `feat/auto-10138`, opened 2026-10-31
17:26 UTC) implements the whole issue:

- `crates/ui-egui/src/i18n.rs` — `SYSTEM` (`"system"`), `match_system_tags` (exact tag,
  then base language, then English), `system_language()` (reads the locale through the
  `sys-locale` crate: OS locale on desktop, `navigator.languages` in the browser),
  `effective_language()`, and `Locale::for_language` resolving `"system"`.
- `crates/ui-egui/src/prefs.rs` — the default becomes `"system"`; `prepare_preferences`
  accepts `"system"` besides the catalog ids.
- `crates/ui-egui/src/prefs/general.rs` — a "System" entry at the top of the language list.
- `crates/ui-egui/src/fonts.rs`, `crates/ui-egui/src/lib.rs`,
  `apps/solidcraft-web/src/main.rs` — resolve `"system"` to its catalog where a catalog id
  is needed (font fallback order, CJK face choice, theme option labels).
- `assets/locales/{en,ar,de,he,ja,ko,vi,zh}.json` — the new `prefs.language_system` message.
- `docs/localization.md` — documents the `system` value and the matching order.
- `Cargo.toml`/`Cargo.lock`/`THIRD-PARTY-LICENSES.txt` — the `sys-locale` dependency.

Tests in that PR: `system_tags_match_the_catalog_exact_then_base_then_english`,
`a_saved_language_choice_is_never_overridden` (both in `crates/ui-egui/src/i18n.rs`) and
`language_dropdown_defaults_to_system` (`crates/ui-egui/src/i18n_ui_tests.rs`).

## The RED test left here

`crates/ui-egui/src/i18n.rs`, in the existing `mod tests` of the area's file:

- `the_language_preference_defaults_to_following_the_system` — the default language is
  `"system"`, not `"en"`.
- `a_saved_language_preference_wins_over_the_system` — a saved `language` is kept.

Both fail against `origin/main` (the default is `"en"`) and pass with the fix.
