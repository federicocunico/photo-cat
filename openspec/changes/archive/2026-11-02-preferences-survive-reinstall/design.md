# Design

## Context

- Data that already survives upgrades and uninstalls: `AppPaths` root (`%APPDATA%\photocat\photo-cat\data`, with `config.toml`, `catalog.db`, thumbnails, models) and `app_dir` (`%APPDATA%\photocat\photo-cat\config`, with `app.db`). The Tauri NSIS uninstaller only deletes `$APPDATA\com.photocat.app` and `$LOCALAPPDATA\com.photocat.app`, and only if the user ticks the checkbox. It never does this in update mode.
- Theme and language are in `config.toml` (`[ui]`) and are applied to the `layout` store at startup (`App.tsx`), so they already survive.
- The persisted zustand stores (`photo-cat.layout`, `photo-cat.library`, `photo-cat.floating`) only write to `localStorage`, which lives in the WebView2 profile in `$LOCALAPPDATA\com.photocat.app\EBWebView`.
- Zustand `persist` hydrates synchronously when the store is created, i.e. when `App` and the stores are imported.

## Goals / Non-Goals

**Goals:** keep layout preferences across uninstall + reinstall, without changing the installer.

**Non-Goals:** syncing between multiple running instances; moving the catalog or changing the data folders; custom NSIS hooks.

## Decisions

### 1. `ui-prefs.json` file in `app_dir`
A JSON object `{ "photo-cat.layout": "<serialized value>", … }` next to `app.db`. `app_dir` belongs to the user and doesn't follow `data_dir`, like identities and tag sets. Atomic write (temporary file + rename). A missing or unreadable file means "no preferences", never an error that blocks startup. Only known keys (`photo-cat.*`) are accepted, to avoid writing arbitrary data.

*Rejected alternative*: put everything in `config.toml`. It would mix interface state that changes often (panel positions) with configuration, and every resize would rewrite the config.

*Rejected alternative*: an NSIS hook that preserves `EBWebView`. That would mean keeping a custom installer template and wouldn't cover a manual deletion of the folder.

### 2. Restore before the app loads
`main.tsx` calls `restoreUiPrefs()` and only then imports `App` dynamically, so the stores hydrate from an already restored `localStorage`. For each key: if `localStorage` lacks it and the file has it, it's copied in. Then the current state is saved to the file (this also covers the first launch after an upgrade). Outside Tauri (tests, browser) it does nothing.

### 3. Storage that mirrors writes
The stores use `createJSONStorage(() => mirroredStorage)`: reads from `localStorage`, writes to `localStorage` and, with a 500 ms debounce, saves all `photo-cat.*` keys to the file. On `pagehide`/`beforeunload` pending saves are flushed right away.

## Risks / Trade-offs

- [A change made less than 500 ms before closing might not reach the file] → flush on `pagehide`; at worst the last change is lost only if the WebView2 profile is also deleted.
- [Async startup delays the first render by one IPC call] → a local file read, a few ms.
- [Interface preferences shared between data folders] → intended: they're per-user preferences, like identities.

## Migration Plan

No migration: at the first launch the file is created from the current state. Older versions ignore it. Rollback: delete `ui-prefs.json`.
