# Dashboard Home Screen Redesign

## Summary

Redesign the `ProfileSelect` home screen from a vertical stack of action cards into a 3-section dashboard layout: hero card, update banner, and tool tiles. The goal is to reduce visual clutter, establish clear information hierarchy, and make better use of horizontal space while keeping the centered column layout.

## Current State

The home screen is a single vertical column containing:
- Centered logo + title + subtitle
- System info banner
- 2+1 profile cards (Laptop, Desktop side-by-side + Manual full-width)
- 5 full-width action cards (Check updates, Uninstall, Search winget, Clone repos, Settings)
- Status footer

This creates a long scrollable list where the primary action (profile selection) competes visually with secondary tools.

## Design

### Layout

All content stays within the existing ~500px centered column. Three sections stack vertically with 10-12px gaps between them.

### Section 1 — Hero Card

A `container` with a gradient background (135° linear gradient from `Color::from_rgb(0.12, 0.16, 0.24)` to `CARD_BG`). This is the outer container only — buttons inside have their own styling.

- **Top row**: Provision logo (blue, `Icon::Package`) + "Provision" title left-aligned. System info string (hostname · OS · CPU · RAM) right-aligned in muted text (`MUTED` color, 9px).
- **Bottom row**: Three profile buttons in a horizontal row with equal widths (`Length::Fill`). Each button shows the profile icon + name. Styled with subtle border (`BORDER`) and translucent background (`Color::from_rgba(1.0, 1.0, 1.0, 0.06)`).
- **Click behavior**: Each profile button triggers `Message::ProfileSelected(profile)` — same as today.
- **Spacing**: 16px padding, 12px gap between top row and profile buttons, 8px gap between profile buttons.

### Section 2 — Update Banner

A full-width card with three zones in a horizontal layout. Clicking "Scan" still navigates to the `UpdateScanning` screen (existing behavior unchanged). The banner states below describe what the user sees *before* they click Scan.

- **Left**: Large green number showing available update count (font-size ~28-32px, bold, `STATUS_GREEN`). When no scan has completed, show a "—" placeholder.
- **Center** (flex:1): "Updates available" as the primary label. Secondary line: "`self.installed_map.len()` installed · `self.catalog.len()` in catalog" in muted text (`MUTED_FG`).
  - When 0 updates: "All up to date" with green checkmark icon (`Icon::CircleCheck`).
  - When scan not done: "Scan to check for updates" in muted text.
- **Right**: Green "Scan" button that triggers `Message::StartUpdateScan`. Styled with `STATUS_GREEN` background, `BG` (`#09090b`) text color. Always visible and enabled.

### Section 3 — Tool Tiles

A 3-column grid of equal-sized cards:

| Tile | Icon | Color | Message |
|------|------|-------|---------|
| Uninstall | `Icon::Trash2` | `STATUS_RED` (red) | `GoToUninstall` |
| Search | `Icon::Search` | `STATUS_AMBER` (amber) | `GoToWingetSearch` |
| Repos | `Icon::Github` | Purple — new constant `REPOS_PURPLE: Color::from_rgb(0.66, 0.33, 0.97)` | `GoToGitHubLogin` |

Each tile contains:
- A colored icon inside a tinted rounded-square container (32×32px, icon color at ~10% opacity for the background, 8px border-radius). Implemented as a `container` with `width(32).height(32).center_x().center_y()` and a background color of `Color::from_rgba(accent.r, accent.g, accent.b, 0.1)`.
- Label text (11px) centered below the icon.
- Card padding: 18px vertical, 10px horizontal.
- Standard card styling with `CARD_BG` background and `BORDER`.
- Each tile is a `button` widget wrapping the icon + label column.

Uninstall tile should be disabled (muted styling) when `installed_packages` is empty and scan isn't done — same logic as current `GoToUninstall` action card.

### Footer

Single row at the bottom:
- **Left**: Catalog status — green package icon + "{count} packages (updated)" when remote, or muted when embedded/cached.
- **Right**: Version string "v{version}" + settings gear icon (`Icon::Settings`). The gear icon is a clickable button triggering `Message::OpenSettings`.

### Update Version Banner

When `latest_release` indicates a newer version, the existing update banner appears **between the hero card and the update banner** (same content and styling as today, just repositioned).

### What Gets Removed

- The vertical action card list (5 cards: Check updates, Uninstall, Search, Clone repos, Settings)
- The separate system info banner (hostname/OS/CPU/RAM) — merged into hero card top-right
- The centered "Select a profile to get started" subtitle

### What Stays the Same

- All other screens (PackageSelect, Review, Installing, UpdateScanning, etc.) — no changes
- `Screen` enum — no new variants
- `Message` enum — no new messages, just rewiring existing ones to new UI elements
- Profile selection logic, catalog fetching, background scans — all unchanged
- Clicking Scan still navigates to `UpdateScanning` screen
- `action_card()` and `profile_card()` helpers kept (may be used elsewhere)

## Implementation Scope

**Files to modify:**
- `src/views.rs` — Rewrite `view_profile_select()`. Add new helper functions for the hero card, update banner, and tool tiles.
- `src/styles.rs` — Add new style functions: `hero_card_style()`, `update_banner_style()` (if not already present), `tool_tile_style()`, `scan_button_style()`. Add `REPOS_PURPLE` color constant. Add `tinted_icon_bg(color: Color) -> Color` helper that returns the color at 10% opacity.
- `DESIGN.md` — Update the "Home Screen" section to reflect the new 3-section dashboard layout.

**Files unchanged:**
- `src/main.rs` — No changes to state, messages, or update logic
- `src/install.rs`, `src/upgrade.rs`, `src/uninstall.rs`, `src/catalog.rs` — No changes
- All other view methods — No changes

## Edge Cases

- **First launch (no scan done)**: Update banner shows "—" for the number, "Scan to check for updates" as subtitle, Scan button enabled.
- **0 updates**: Update banner shows green checkmark icon, "All up to date" label. Scan button still available for re-scan.
- **Catalog still loading**: Stats line shows "Loading..." for catalog count. Profile buttons still work (they use embedded catalog as fallback).
- **Window too narrow**: The 3-column tile grid should use `Length::Fill` on each tile so they compress gracefully. Profile buttons in the hero similarly use `Length::Fill`.
