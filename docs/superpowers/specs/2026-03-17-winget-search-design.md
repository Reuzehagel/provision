# Winget Search Integration

**Date:** 2026-03-17
**Status:** Approved

## Summary

A dedicated "Search Winget" screen where users can search the full winget catalog (`winget search`), view results (name, ID, version), multi-select packages, and install them via the existing progress screen. This is a separate flow from the curated provisioning — intended as an ongoing "grab something" tool rather than initial machine setup.

## Data Model

### `SearchPackage` struct (in `upgrade.rs`)

```rust
pub struct SearchPackage {
    pub name: String,
    pub winget_id: String,
    pub version: String,
    pub source: String,
    pub name_lower: String,       // precomputed for filtering
    pub winget_id_lower: String,  // precomputed for filtering
}
```

- Mirrors `InstalledPackage` minus `size_bytes`
- Implements `BatchItem` for use with `run_winget_batch()`
- Follows the precomputed lowercase field pattern used throughout the codebase

### `SearchProgress` enum (in `upgrade.rs`)

```rust
pub enum SearchProgress {
    Activity { line: String },
    Log { line: String },
    Completed { packages: Vec<SearchPackage> },
    Failed { error: String },
}
```

Same pattern as `ScanProgress` and `InstalledScanProgress`.

## Screens

Two new `Screen` variants:

- **`WingetSearch`** — search input + results list with checkboxes
- **`WingetSearchInstalling`** — reuses `view_progress_screen()`

### Screen Flow

1. `ProfileSelect` → click "Search Winget" card → `WingetSearch`
2. User types query, presses Enter or clicks Search button → streams `winget search <query>` results
3. User checks packages, clicks "Install Selected" → `WingetSearchInstalling`
4. Done → back to `WingetSearch` (not ProfileSelect, so they can search again)

## App State

New fields on `App`:

```rust
pub(crate) winget_search_query: String,
pub(crate) winget_search_results: Vec<SearchPackage>,
pub(crate) winget_search_selected: HashSet<String>,  // by winget_id
pub(crate) winget_search_scanning: bool,
pub(crate) winget_search_error: Option<String>,
pub(crate) winget_search_queue: Vec<SearchPackage>,
pub(crate) winget_search_install: ProgressState,
pub(crate) _winget_search_handle: Option<task::Handle>,
```

## Search Engine

### `search_winget(query, dry_run)` (in `upgrade.rs`)

Returns `impl Stream<Item = SearchProgress>`. Uses `run_winget_scan()` with args `["search", &query, "--accept-source-agreements"]`.

- **Dry run:** returns fake results after a short delay
- **Real:** spawns winget, streams output, parses with `parse_search_table()`

### `parse_search_table(lines)` (in `upgrade.rs`)

Parses `winget search` column-aligned output. Columns: `Name`, `Id`, `Version`, `Source` — same structure as `parse_list_table()`.

### Installation

Thin wrapper over `run_winget_batch()` with:
- Verb: `"install"`
- Base args: `["--accept-package-agreements", "--accept-source-agreements"]`

Same pattern as `upgrade_all()`.

### No debouncing

Search fires only on explicit action (Enter key or button), not per-keystroke. Avoids hammering winget.

## Messages

```rust
GoToWingetSearch,
WingetSearchQueryChanged(String),
StartWingetSearch,
WingetSearchProgress(SearchProgress),
ToggleWingetSearchPackage(String),
StartWingetSearchInstall,
CancelWingetSearchInstall,
WingetSearchInstallProgress(install::InstallProgress),
FinishWingetSearchInstall,
SelectAllWingetSearch,
```

### Handler Behavior

| Message | Action |
|---------|--------|
| `GoToWingetSearch` | Set screen, clear previous results/selection, focus search input |
| `WingetSearchQueryChanged` | Store query string (no search triggered) |
| `StartWingetSearch` | Spawn `search_winget()` stream, store handle, set `scanning = true` |
| `WingetSearchProgress` | Activity → spinner, Completed → populate results + auto-select all, Failed → set error |
| `StartWingetSearchInstall` | Build queue from selected, call `run_winget_batch()`, transition to `WingetSearchInstalling` |
| `FinishWingetSearchInstall` | Clear install state, return to `WingetSearch` |
| `SelectAllWingetSearch` | Toggle all visible results via `toggle_set()` |

### Keyboard

- **Enter:** triggers search on `WingetSearch` (if not scanning); triggers done on `WingetSearchInstalling` when complete
- **Escape:** goes back
- **Ctrl+K:** focuses search input
- All consistent with existing screen behavior

## View Layout

### `view_winget_search()`

Structure mirrors `view_uninstall_select()`:

1. **Header row:** back arrow + "Search Winget" title
2. **Search bar:** text input + "Search" button (disabled while scanning). Enter triggers search
3. **Results area:** scrollable list with checkbox rows showing name, winget ID, version
4. **Empty states:**
   - Before first search: "Type a query and press Enter"
   - No results: "No results found"
   - Scanning: spinner + "Searching..."
   - Error: error message
5. **Footer:** "Install Selected (N)" button + "Select All" button

### `WingetSearchInstalling`

Reuses `view_progress_screen()` directly. "Done" button routes to `FinishWingetSearchInstall` (back to search screen).

### ProfileSelect Entry Point

New `action_card()` on the home dashboard:
- Icon: `Icon::Search`
- Title: "Search Winget"
- Description: "Find and install any package from the winget catalog"
- Placed alongside existing Update and Uninstall cards

## Files Modified

| File | Changes |
|------|---------|
| `src/upgrade.rs` | `SearchPackage`, `SearchProgress`, `search_winget()`, `parse_search_table()` |
| `src/main.rs` | `Screen` variants, `App` state fields, `Message` variants, handler methods, keyboard routing |
| `src/views.rs` | `view_winget_search()`, `view_winget_search_installing()` (or reuse `view_progress_screen`), ProfileSelect card |

No new files. No new dependencies. No new style functions needed.

## Approach

Streamed search (Approach A): run `winget search` as a streamed process via `run_winget_scan()` + column parser, consistent with existing upgrade scan and installed scan patterns.
