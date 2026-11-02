# Tasks

## 1. Backend

- [x] 1.1 Create `app/src-tauri/src/ui_prefs.rs` with read (missing/corrupt file → empty map) and atomic write of `ui-prefs.json` in `AppPaths::app_dir`, accepting only `photo-cat.*` keys; verify with unit tests (round trip, corrupt file, keys discarded)
- [x] 1.2 Add and register the `ui_prefs_load` and `ui_prefs_save` commands, regenerate `bindings.ts` with `cargo test -p photo-cat --test bindings`; verify that `npm run typecheck` passes

## 2. UI

- [x] 2.1 Create `app/ui/src/lib/uiPrefs.ts` (`restoreUiPrefs`, mirrored storage with 500 ms debounce and flush on `pagehide`) and use it in the `layout`, `library` and `floating` stores; verify with a vitest test (restore only missing keys, current state saved to the file, debounced writes)
- [x] 2.2 In `main.tsx` call `restoreUiPrefs()` before dynamically importing `App`; verify that `npm test` and `npm run build` pass

## 3. Documentation and checks

- [x] 3.1 README, "Aggiornare e tornare a una versione precedente" section: what upgrades and uninstalls keep (folders, "Delete app data" checkbox) and how to remove everything; check that the paths match `AppPaths`
- [x] 3.2 Run `cargo test`, `npm test` and `make test-linux`, fixing every failure
- [ ] 3.3 Real installer test on Windows, with a backup of the user data first: install the new build over the installed version (prefs and library unchanged), then uninstall with "Delete app data" and reinstall (prefs restored from `ui-prefs.json`, library intact)

## Workflow follow-up

- PR, merge to main after CI, archive and sync the `desktop-shell` spec.
