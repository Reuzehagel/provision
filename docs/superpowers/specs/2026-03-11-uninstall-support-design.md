# Uninstall Support — Design Spec

## Overview

Add a full uninstall flow to the provisioning app. Users can browse all installed packages (from `winget list`), select packages to remove, confirm the selection, and watch uninstall progress with streaming terminal output. Package sizes are loaded asynchronously from the Windows registry.

Designed for future extensibility — leftover file/registry cleanup can be added later without refactoring.

## Screen Flow

```
ProfileSelect → UninstallSelect → UninstallReview → Uninstalling → (re-scan) → ProfileSelect
```

- **No scanning step** — installed package data is already loaded at startup via `scan_installed()`
- Entry point: "Uninstall packages" card on the home screen, peer to "Check for updates"
- Back navigation: `UninstallSelect` → `ProfileSelect`, `UninstallReview` → `UninstallSelect` (preserving selection state)
- After completion, re-runs `scan_installed()` to refresh the installed packages map
- Clear `self.search` when navigating to `UninstallSelect` (shared search state convention)

## New Screen Variants

```rust
enum Screen {
    // ... existing variants ...
    UninstallSelect,   // Browse & select installed packages
    UninstallReview,   // Confirm before uninstalling
    Uninstalling,      // Progress screen
}
```

## Data Model

### Enriched `InstalledPackage` (in `upgrade.rs`)

The existing `InstalledPackage` struct (currently only `winget_id` and `version`) gains new fields:

```rust
#[derive(Debug, Clone)]
pub struct InstalledPackage {
    pub name: String,
    pub winget_id: String,         // original case (needed for winget uninstall --id)
    pub version: String,
    pub source: String,
    pub winget_id_lower: String,   // precomputed for search & is_installed() lookups
    pub name_lower: String,        // precomputed for search
    pub size_bytes: Option<u64>,   // filled async from registry
}
```

**Required parser change:** `parse_list_table()` currently only extracts the `Id` column (lowercased) and `Version`. It must be updated to also extract the `Name` and `Source` columns (column positions are already parsed from the header, similar to how `parse_upgrade_table()` works). The `winget_id` field must store the **original case** from the table (needed for `winget uninstall --id`), with the lowercased version in `winget_id_lower`.

### Changed `App::installed`

Currently `HashMap<String, String>` (winget_id_lower → version). Changes to store richer data:

```rust
pub installed_packages: Vec<InstalledPackage>,          // full data for uninstall screen
pub installed_map: HashMap<String, String>,             // winget_id_lower → version (derived from vec, for O(1) is_installed() lookups)
pub installed_scan_done: bool,                          // existing field, unchanged
```

The `installed_map` is built from `installed_packages` after each scan completes, preserving the existing `is_installed()` behavior unchanged. Both are populated together from the same scan result.

### New App State

```rust
// Uninstall flow
pub uninstall_selected: HashSet<String>,    // selected winget_id_lower values
pub uninstall_queue: Vec<InstalledPackage>, // packages queued for removal (Clone into stream)
pub uninstall: ProgressState,              // reuses existing ProgressState
pub size_scan_done: bool,                  // registry size scan complete
```

### New Messages

```rust
GoToUninstall,                             // navigate to UninstallSelect
ToggleUninstallPackage(String),            // toggle individual package
GoToUninstallReview,                       // advance to review
StartUninstall,                            // begin uninstalling
CancelUninstall,                           // cancel mid-uninstall
UninstallProgress(InstallProgress),        // stream events (reuses existing enum)
FinishUninstallAndReset,                   // done → re-scan → home
SizeScanResult(Vec<(String, u64)>),        // async registry results (winget_id_lower → bytes)
```

`InstallProgress` (Started/Log/Activity/Succeeded/Failed/Completed) is reused — uninstall progress has the exact same shape.

## New Module: `uninstall.rs`

### `uninstall_all()`

Execution stream, same pattern as `upgrade_all()`:

- Takes `Vec<InstalledPackage>`, `dry_run: bool`, `extra_args: Vec<String>`
- For each package: builds args `["uninstall", "--id", &pkg.winget_id, "-e", "--accept-source-agreements", ...extra_args]`
- Calls `install::run_command("winget", &args, index, &mut sender)` — same helper that `upgrade_all()` uses. Handles process spawning, `CREATE_NO_WINDOW`, stdout streaming, and output classification.
- Streams `InstallProgress` events through `iced::stream::channel`
- In dry-run mode: simulates with a sleep (same pattern as upgrade)

### `scan_sizes()`

Async registry lookup for installed package sizes:

- Reads from `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall`
- Also reads `HKCU\...` and `WOW6432Node` paths (32-bit apps on 64-bit Windows)
- Extracts `EstimatedSize` DWORD (in KB, converted to bytes) and `DisplayName` for matching
- Takes `&[InstalledPackage]` as input to match registry entries against known packages
- Returns `Vec<(String, u64)>` — winget_id_lower → size in bytes
- **Matching strategy:** For each registry entry, try these in order:
  1. Exact match: registry `DisplayName` equals `InstalledPackage.name` (case-insensitive)
  2. ID-based: registry key name or `DisplayName` contains the winget ID segments (e.g., "Git" from "Git.Git")
  3. If no match, skip — `Option<u64>` handles misses gracefully
- Runs as a background `Task::perform` kicked off after the installed scan completes, does not block any screen

### Winget Flags for Uninstall

Subset of `install_args()` that applies to uninstall:

- `--silent` / `--interactive` — yes
- `--force` — yes
- `--disable-interactivity` — yes
- `--scope` — no (uninstalling what's already there)
- `--architecture` — no
- `--ignore-security-hash` — no
- `--location` — no
- `--accept-package-agreements` — no (install/upgrade only)
- `--accept-source-agreements` — included defensively (harmless, avoids edge-case prompts)

A new `uninstall_args()` method on `WingetSettings` returns only the relevant flags.

## View Layer

### UninstallSelect Screen

Table layout with column headers:

| Checkbox | Name | Version | Size | Package ID |
|----------|------|---------|------|------------|

- Flat list sorted alphabetically by `name` (fallback to `winget_id` if name is empty)
- Search filters by name and winget ID (using precomputed lowercase fields)
- Size shows "—" when registry data unavailable
- Selected rows get highlighted background
- Ctrl+A selects/deselects all (respects search filter)
- Footer: "N selected · ~X MB" + "Review uninstall" button (red accent, using `STATUS_RED` / `#ef4444`)
- Use Lucide icons for navigation (no unicode arrows)

### UninstallReview Screen

Confirmation step before execution:

- Amber warning banner: "This action cannot be undone" with note that packages can be reinstalled via winget
- List of selected packages with X icon, name, winget ID, version, size
- Total freed space estimate: "Estimated ~X MB will be freed" (sum of known sizes; not all packages report size)
- Footer: "Edit" button + "Uninstall N packages" button (red accent)

### Uninstalling Screen

Reuses existing `view_progress_screen()` template with uninstall-specific labels:

- Heading: "Uninstalling X of Y" → "Uninstall Complete"
- Subtitle: "Removing {package name}..." → result summary
- Progress bar, per-package status list, terminal log box
- Cancel button (same behavior as install/upgrade: current package finishes, rest cancelled)
- Copy log + Done button when complete

### Home Screen Addition

New "Uninstall packages" card alongside "Check for updates":

- Same visual weight and style as the updates card
- Shows installed count if scan is complete
- Disabled state if installed scan hasn't completed or returned empty

### Color Accent

Red (`STATUS_RED` / `#ef4444`) for destructive uninstall buttons, consistent with the existing `STATUS_RED` constant in `styles.rs`. Warning banner uses `STATUS_AMBER`.

## Startup Changes

### Enhanced Installed Scan

The existing `scan_installed()` at startup already parses the full `winget list` table — all column positions (Name, Id, Version, Source) are derivable from the header. `parse_list_table()` must be updated to extract all four columns and preserve original-case winget IDs. The `InstalledPackage` struct gains `name`, `source`, `name_lower`, `winget_id_lower`, and `size_bytes: None` fields.

After the scan completes:
1. Store full `Vec<InstalledPackage>` in `installed_packages`
2. Build `installed_map: HashMap<String, String>` from `winget_id_lower → version` (preserves `is_installed()` behavior)

### Background Size Scan

After the installed scan completes, kick off `scan_sizes()` as a separate background `Task::perform`. When results arrive via `SizeScanResult`, merge sizes into the `installed_packages` vec by matching `winget_id_lower`. UI shows "—" for packages until (and if) their size resolves.

## Post-Uninstall Behavior

After uninstall completes and the user clicks Done:

1. Re-run `scan_installed()` to refresh installed packages data
2. Navigate back to `ProfileSelect`
3. "Installed" badges on the install screen reflect the updated state
4. Kick off a new background size scan for the refreshed list

## Cancellation

Same pattern as install/upgrade:

- Cancel stops queuing new packages
- Currently-running uninstall finishes
- Remaining packages marked "Cancelled"
- No extra warning beyond the standard cancel behavior

## Error Handling

1. **No installed packages**: "Uninstall packages" card disabled or shows "0 installed". Empty state message on UninstallSelect screen.
2. **Winget uninstall fails**: Package marked "Failed" with error message, continues to next package.
3. **Package requires elevation**: Winget handles UAC prompts. App streams output normally.
4. **Registry size misses**: Displayed as "—". No error state needed.
5. **Post-uninstall re-scan fails**: Keep stale data rather than clearing. App still works.
6. **Dry-run mode**: Respects `--dry` flag. Simulates uninstall without removing anything.

## Extensibility for Future Cleanup

The design accommodates a future "leftover cleanup" phase:

- `InstalledPackage` can gain fields like `registry_keys: Vec<String>`, `install_dirs: Vec<PathBuf>`
- `uninstall.rs` module is the natural home for cleanup scanning logic
- A post-uninstall "Cleanup" screen could slot between `Uninstalling` and the return to `ProfileSelect`
- The review screen's warning banner can be extended with cleanup options

No code for cleanup is included in this iteration.
