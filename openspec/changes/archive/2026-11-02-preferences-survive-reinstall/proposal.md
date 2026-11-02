# Proposal

## Why

Installing a new version over an existing one must not lose the user's preferences, and neither must uninstalling and reinstalling. Settings, catalog, identities and tag sets already survive: they live in the app's user folder (`%APPDATA%\photocat\photo-cat` on Windows), which the installer never touches. Theme and language are also kept in `config.toml`. Layout preferences, however, live only in the WebView2 storage (`%LOCALAPPDATA%\com.photocat.app`): active module, sidebar open state and width, grid sort order and thumbnail size, folder-filter mode, and floating panel positions and sizes. The Windows uninstaller deletes that folder when the user ticks "Delete app data", so after reinstalling these preferences go back to their defaults.

## What Changes

- Layout preferences are also saved to a file in the app folder, next to the identity and tag-set archive (`ui-prefs.json`). It doesn't follow the data path, and the uninstaller doesn't delete it.
- At startup, if the WebView2 storage is missing a preference (new or wiped profile), the app restores it from the file before showing the interface. If both exist, the WebView2 storage wins, and the file is updated with it.
- On the first launch of the new version, existing preferences are copied to the file right away, so a later uninstall doesn't lose them.
- The README explains what upgrades and uninstalls keep and how to remove everything.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `desktop-shell`: new requirement "Preferenze conservate tra installazioni".

## Impact

- **Rust**: new `ui_prefs` module in `app/src-tauri` (read and atomic write of `ui-prefs.json` in `AppPaths::app_dir`), commands `ui_prefs_load` and `ui_prefs_save`, regenerated `bindings.ts`.
- **UI**: `app/ui/src/lib/uiPrefs.ts` (restore at startup, storage that mirrors writes to the file), `main.tsx` (restore before the app loads), the persisted stores (`layout`, `library`, `floating`) use the new storage.
- **Installer**: no change to the Tauri template.
- **Compatibility**: a new file only, no migration. Older versions ignore it.
