# Dashboard Home Screen Redesign — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign the ProfileSelect home screen into a 3-section dashboard: hero card, update banner, and tool tiles.

**Architecture:** View-only change — rewrite `view_profile_select()` in `src/views.rs` and add supporting styles in `src/styles.rs`. No changes to state, messages, or update logic in `src/main.rs`.

**Tech Stack:** Rust, Iced 0.14, lucide-icons crate

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `src/styles.rs` | Modify | Add `REPOS_PURPLE` constant, `tinted_icon_bg()` helper, `hero_card_style()`, `scan_button_style()`, `tool_tile_style()`, `hero_profile_button_style()` |
| `src/views.rs` | Modify | Rewrite `view_profile_select()`, add `hero_card()`, `update_banner()`, `tool_tile()` helper functions |
| `DESIGN.md` | Modify | Update "Home Screen (Profile Select)" section to match new layout |

---

### Task 1: Add new color constants and style helpers to `src/styles.rs`

**Files:**
- Modify: `src/styles.rs:44-65` (after accent colors section)
- Modify: `src/styles.rs:293` (after `update_card_style`)

- [ ] **Step 1: Add `REPOS_PURPLE` color constant**

Add after `STATUS_AMBER` (line 65) in `src/styles.rs`:

```rust
pub const REPOS_PURPLE: Color = Color::from_rgb(0.66, 0.33, 0.97);
```

- [ ] **Step 2: Add `BG` color constant**

Add at the top of the zinc neutral palette section (after line 7) in `src/styles.rs`. This is used for the Scan button text color:

```rust
pub const BG: Color = Color::from_rgb(
    0x09 as f32 / 255.0,
    0x09 as f32 / 255.0,
    0x0b as f32 / 255.0,
); // zinc-950 — app background
```

- [ ] **Step 3: Add `tinted_icon_bg()` helper**

Add after the accent colors section in `src/styles.rs`:

```rust
/// Returns a color at 10% opacity, for tinted icon circle backgrounds.
pub fn tinted_icon_bg(color: Color) -> Color {
    Color::from_rgba(color.r, color.g, color.b, 0.1)
}
```

- [ ] **Step 4: Add `hero_card_style()` container style**

Add after `divider_style` (line 391) in `src/styles.rs`. Uses a gradient background from a dark slate-blue to `CARD_BG`:

```rust
pub fn hero_card_style(_theme: &Theme) -> container::Style {
    use iced::gradient::{self, Linear};
    use iced::Radians;

    let gradient = Linear::new(Radians(135.0_f32.to_radians()))
        .add_stop(0.0, Color::from_rgb(0.12, 0.16, 0.24))
        .add_stop(1.0, CARD_BG);

    container::Style {
        background: Some(Background::Gradient(gradient::Gradient::Linear(gradient))),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}
```

Note: If `iced::gradient` is not available or the gradient API doesn't match, fall back to a solid color:

```rust
pub fn hero_card_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb(0.12, 0.14, 0.18))),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}
```

- [ ] **Step 5: Add `hero_profile_button_style()`**

Add after `hero_card_style` in `src/styles.rs`. Translucent white background with subtle border:

```rust
pub fn hero_profile_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.12),
        button::Status::Pressed => Color::from_rgba(1.0, 1.0, 1.0, 0.15),
        _ => Color::from_rgba(1.0, 1.0, 1.0, 0.06),
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: TEXT,
        border: Border {
            color: BORDER_FOCUS,
            width: 1.0,
            radius: 6.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}
```

- [ ] **Step 6: Add `scan_button_style()`**

Add after `hero_profile_button_style` in `src/styles.rs`. Green background with dark text:

```rust
pub fn scan_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(
            0x0d as f32 / 255.0,
            0x9e as f32 / 255.0,
            0x6f as f32 / 255.0,
        ), // slightly darker emerald
        button::Status::Pressed => Color::from_rgb(
            0x0a as f32 / 255.0,
            0x84 as f32 / 255.0,
            0x5d as f32 / 255.0,
        ),
        _ => STATUS_GREEN,
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: BG,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}
```

- [ ] **Step 7: Add `tool_tile_style()`**

Add after `scan_button_style` in `src/styles.rs`. Standard card button with 8px radius:

```rust
pub fn tool_tile_style(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => CARD_HOVER,
        _ => CARD_BG,
    };
    let border_color = match status {
        button::Status::Hovered => BORDER_FOCUS,
        _ => BORDER,
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: TEXT,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}
```

- [ ] **Step 8: Run `just check`**

Run: `just check`
Expected: Build succeeds, clippy clean, fmt clean. New styles compile but aren't used yet (dead code warnings are OK at this point — they'll resolve in Task 2).

- [ ] **Step 9: Commit**

```bash
git add src/styles.rs
git commit -m "feat: add dashboard home screen styles (hero, scan button, tool tiles)"
```

---

### Task 2: Rewrite `view_profile_select()` in `src/views.rs`

**Files:**
- Modify: `src/views.rs:1-23` (imports)
- Modify: `src/views.rs:26-203` (replace `view_profile_select`)

- [ ] **Step 1: Update imports in `src/views.rs`**

Add the new style imports to the existing import block at `src/views.rs:16-22`. Add `REPOS_PURPLE`, `BG`, `hero_card_style`, `hero_profile_button_style`, `scan_button_style`, `tool_tile_style`, `tinted_icon_bg` to the `use crate::styles::{...}` statement. Also add `CARD_HOVER` and `BORDER_FOCUS` if not already imported. No need to add `Font` — the title uses the default font, so omit the `.font()` call.

- [ ] **Step 2: Write the new `view_profile_select()` method**

Replace the entire `view_profile_select` method body (`src/views.rs:26-203`) with the new dashboard layout. The structure is:

```rust
pub(crate) fn view_profile_select(&self) -> Element<'_, Message> {
    // ── Section 1: Hero Card ────────────────────────────────
    let si = &self.system_info;

    let logo = text(char::from(Icon::Package))
        .size(16)
        .font(LUCIDE_FONT)
        .color(STATUS_BLUE);
    let title = text("Provision").size(16);
    let brand = row![logo, title].spacing(6).align_y(iced::Alignment::Center);

    let sys_info_text = text(format!(
        "{} · {} · {:.0} GB",
        si.hostname, si.cpu_name, si.ram_gb
    ))
    .size(9)
    .color(MUTED);

    let hero_top = row![
        brand,
        iced::widget::Space::new().width(Length::Fill),
        sys_info_text,
    ]
    .align_y(iced::Alignment::Center);

    // Profile buttons
    let profile_buttons: Vec<Element<'_, Message>> = Profile::ALL
        .iter()
        .map(|&p| {
            let icon = text(p.icon())
                .size(12)
                .font(LUCIDE_FONT);
            let label = text(p.title()).size(11);
            let content = row![icon, label]
                .spacing(6)
                .align_y(iced::Alignment::Center);

            button(
                container(content)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .padding([8, 4]),
            )
            .on_press(Message::ProfileSelected(p))
            .width(Length::Fill)
            .style(hero_profile_button_style)
            .into()
        })
        .collect();

    let profile_row = iced::widget::Row::with_children(profile_buttons)
        .spacing(8)
        .width(Length::Fill);

    let hero = container(
        column![hero_top, profile_row]
            .spacing(12)
            .width(Length::Fill),
    )
    .padding(16)
    .width(Length::Fill)
    .style(hero_card_style);

    // ── Section 2: Update Banner ────────────────────────────
    let update_count = self.update_scan.packages.len();
    let scan_done = self.update_scan.done;

    let count_display: Element<'_, Message> = if scan_done {
        if update_count > 0 {
            text(format!("{update_count}"))
                .size(28)
                .color(STATUS_GREEN)
                .into()
        } else {
            text(char::from(Icon::CircleCheck))
                .size(24)
                .font(LUCIDE_FONT)
                .color(STATUS_GREEN)
                .into()
        }
    } else {
        text("—").size(28).color(MUTED).into()
    };

    let update_label: Element<'_, Message> = if scan_done && update_count == 0 {
        text("All up to date").size(12).color(TEXT).into()
    } else if scan_done {
        text("Updates available").size(12).color(TEXT).into()
    } else {
        text("Scan to check for updates")
            .size(12)
            .color(MUTED_FG)
            .into()
    };

    let installed_count = self.installed_map.len();
    let catalog_count = self.catalog.len();
    let stats_text = if !self.installed_scan_done && installed_count == 0 {
        "Scanning installed...".to_string()
    } else {
        format!("{installed_count} installed · {catalog_count} in catalog")
    };
    let stats_line = text(stats_text).size(10).color(MUTED_FG);

    let update_info = column![update_label, stats_line].spacing(2);

    let scan_btn = button(
        text("Scan").size(11),
    )
    .on_press(Message::StartUpdateScan)
    .style(scan_button_style)
    .padding([7, 16]);

    let update_banner = container(
        row![
            count_display,
            update_info,
            iced::widget::Space::new().width(Length::Fill),
            scan_btn,
        ]
        .spacing(16)
        .align_y(iced::Alignment::Center)
        .width(Length::Fill),
    )
    .padding(16)
    .width(Length::Fill)
    .style(|theme: &Theme| {
        container::Style {
            background: Some(iced::Background::Color(CARD_BG)),
            border: Border {
                color: BORDER,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        }
    });

    // ── Section 3: Tool Tiles ───────────────────────────────
    let uninstall_msg = if self.installed_scan_done && !self.installed_packages.is_empty() {
        Some(Message::GoToUninstall)
    } else {
        None
    };

    let tile_uninstall = tool_tile(Icon::Trash2, "Uninstall", STATUS_RED, uninstall_msg);
    let tile_search = tool_tile(
        Icon::Search,
        "Search",
        STATUS_AMBER,
        Some(Message::GoToWingetSearch),
    );
    let tile_repos = tool_tile(
        Icon::Github,
        "Repos",
        REPOS_PURPLE,
        Some(Message::GoToGitHubLogin),
    );

    let tiles = row![tile_uninstall, tile_search, tile_repos]
        .spacing(8)
        .width(Length::Fill);

    // ── Footer ──────────────────────────────────────────────
    let pkg_count = self.catalog.len();
    let catalog_color = if self.catalog_source == CatalogSource::Remote {
        STATUS_GREEN
    } else {
        MUTED
    };
    let catalog_label = match self.catalog_source.label_suffix() {
        Some(suffix) => format!("{pkg_count} packages ({suffix})"),
        None => format!("{pkg_count} packages"),
    };
    let catalog_status = status_indicator(Icon::Package, catalog_label, catalog_color);

    let version_label = text(format!("v{}", env!("CARGO_PKG_VERSION")))
        .size(9)
        .color(MUTED);

    let settings_icon = button(
        text(char::from(Icon::Settings))
            .size(13)
            .font(LUCIDE_FONT)
            .color(MUTED),
    )
    .on_press(Message::OpenSettings)
    .style(ghost_button_style)
    .padding([2, 4]);

    let footer = row![
        catalog_status,
        iced::widget::Space::new().width(Length::Fill),
        version_label,
        settings_icon,
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    // ── Assemble ────────────────────────────────────────────
    let mut content = column![hero]
        .spacing(10)
        .max_width(500);

    // Update version banner (between hero and update banner)
    if let Some(release) = &self.latest_release {
        let banner_icon = text(char::from(Icon::CircleArrowUp))
            .size(15)
            .font(LUCIDE_FONT)
            .color(STATUS_AMBER);
        let banner_text = text(format!("v{} available", release.version))
            .size(14)
            .color(TEXT);
        let banner_link = text("View release →").size(13).color(STATUS_AMBER);
        let dismiss_icon = text(char::from(Icon::X))
            .size(14)
            .font(LUCIDE_FONT)
            .color(MUTED_FG);
        let dismiss_btn = button(dismiss_icon)
            .on_press(Message::DismissUpdateBanner)
            .style(ghost_button_style)
            .padding([4, 6]);

        let banner_content = row![
            banner_icon,
            banner_text,
            banner_link,
            iced::widget::Space::new().width(Length::Fill),
            dismiss_btn,
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .padding([12, 16])
        .width(Length::Fill);

        let banner = button(banner_content)
            .on_press(Message::OpenReleasePage)
            .width(Length::Fill)
            .style(update_banner_style);

        content = content.push(banner);
    }

    let content = content
        .push(update_banner)
        .push(tiles)
        .push(footer);

    container(content)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .padding(32)
        .into()
}
```

**Important implementation notes:**
- The `container::Style` closure for `update_banner` needs `use iced::Border;` in scope — it should already be available via existing imports.
- `Profile::icon()` returns a `char` directly (not `lucide_icons::Icon`). Use `text(p.icon())` — no `char::from()` wrapper needed.
- If `update_scan.done` is false and no scan has been run, `update_scan.packages` will be empty. The "—" placeholder handles this correctly.

- [ ] **Step 3: Add `tool_tile()` helper function**

Add as a free function after the existing helpers (near `action_card` at line ~2228) in `src/views.rs`:

```rust
fn tool_tile<'a>(
    icon: Icon,
    label: &'a str,
    color: Color,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let icon_bg = container(
        text(char::from(icon))
            .size(14)
            .font(LUCIDE_FONT)
            .color(color),
    )
    .width(32)
    .height(32)
    .center_x(32)
    .center_y(32)
    .style(move |_theme: &Theme| container::Style {
        background: Some(iced::Background::Color(tinted_icon_bg(color))),
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    });

    let content = column![icon_bg, text(label).size(11)]
        .spacing(8)
        .align_x(iced::Alignment::Center);

    let mut btn = button(
        container(content)
            .padding([18, 10])
            .center_x(Length::Fill),
    )
    .width(Length::Fill)
    .style(tool_tile_style);

    if let Some(msg) = on_press {
        btn = btn.on_press(msg);
    }

    btn.into()
}
```

- [ ] **Step 4: Run `just check`**

Run: `just check`
Expected: Build succeeds, clippy clean, fmt clean. The dashboard renders correctly. Fix any compilation errors — common issues:
- Missing imports (add any new items to the `use crate::styles::{...}` block)
- Lifetime issues on `tool_tile` (the `'a` lifetime ties the `label` borrow to the returned `Element`)
- `Border` not in scope inside closures (add `use iced::Border;` at the top of the file or use fully qualified `iced::Border`)

- [ ] **Step 5: Visual smoke test**

Run: `cargo run -- --dry`
Verify:
- Hero card shows with gradient/dark background, Provision branding, system info, 3 profile buttons
- Update banner shows "—" with "Scan to check for updates" and green Scan button
- Three tool tiles (Uninstall, Search, Repos) with colored icon circles
- Footer shows catalog status and version + settings gear
- Clicking profile buttons navigates to package select
- Clicking Scan navigates to update scanning screen
- Clicking tool tiles navigates to correct screens
- Uninstall tile is disabled if no installed packages detected yet

- [ ] **Step 6: Commit**

```bash
git add src/views.rs
git commit -m "feat: rewrite home screen as dashboard with hero, update banner, and tool tiles"
```

---

### Task 3: Update `DESIGN.md`

**Files:**
- Modify: `DESIGN.md:95-265` (Components → Home Screen sections)

- [ ] **Step 1: Update the Home Screen section in DESIGN.md**

Replace the "Profile Cards (Home Screen)" section (lines 95-102), "Update Card (Home Screen)" section (lines 104-108), and "Home Screen (Profile Select)" layout diagram (lines 237-265) with the new dashboard layout description:

Replace lines 95-108 with:

```markdown
### Home Screen — Dashboard Layout

Three-section dashboard within a centered ~500px column:

**Hero Card** (gradient container):
- Top row: Provision logo + title (left), system info (right, muted 9px)
- Bottom row: 3 equal-width profile buttons (Laptop, Desktop, Manual)
- Background: 135° gradient from dark slate-blue to `CARD`
- Padding: 16px, border: 1px `BORDER`, radius: 8px

**Update Banner** (full-width card):
- Left: large update count (28px, `SUCCESS`) or "—" placeholder
- Center: label + stats line (installed/catalog counts)
- Right: green Scan button (`SUCCESS` bg, `BG` text)
- Padding: 16px, same card styling as other cards

**Tool Tiles** (3-column grid):
- Uninstall (red), Search (amber), Repos (purple)
- Each: colored icon in tinted rounded-square (32×32, 8px radius), label below
- Each tile is a button with `CARD` bg, 1px `BORDER`, 8px radius

**Footer**:
- Left: catalog status indicator
- Right: version label + settings gear button
```

Replace the ASCII layout diagram (lines 237-265) with:

```markdown
### Home Screen (Dashboard)

```
┌──────────────────────────────────────┐
│  padding: 32px                       │
│  ┌──────────────────────────────────┐│
│  │ ◆ Provision     hostname · cpu  ││
│  │ [Laptop] [Desktop] [Manual]     ││
│  └──────────────────────────────────┘│
│                                      │
│  ┌──────────────────────────────────┐│
│  │  3   Updates available    [Scan]││
│  │      188 installed · 93 catalog ││
│  └──────────────────────────────────┘│
│                                      │
│  ┌──────────┐┌──────────┐┌─────────┐│
│  │  ⊘       ││  ⌕       ││  ⑂      ││
│  │ Uninstall││ Search   ││ Repos   ││
│  └──────────┘└──────────┘└─────────┘│
│                                      │
│  ✓ 93 packages        v0.3.2 · ⚙   │
└──────────────────────────────────────┘
```
```

- [ ] **Step 2: Run `just check`**

Run: `just check`
Expected: Still passes (DESIGN.md is documentation only).

- [ ] **Step 3: Commit**

```bash
git add DESIGN.md
git commit -m "docs: update DESIGN.md home screen section for dashboard layout"
```

---

### Task 4: Final polish and cleanup

**Files:**
- Possibly modify: `src/views.rs`, `src/styles.rs`

- [ ] **Step 1: Run `cargo fmt`**

Run: `just fmt`
Expected: Any formatting issues auto-fixed.

- [ ] **Step 2: Run `just check` one final time**

Run: `just check`
Expected: Build + clippy + fmt all pass clean.

- [ ] **Step 3: Check for dead code warnings**

Review clippy output. Both `action_card()` and `profile_card()` were only called from `view_profile_select()` and are now dead code. Remove them both, along with any unused style imports they depended on (e.g., `update_card_style` if only used by `action_card`). Also remove the `CARD_HOVER` and `BORDER_FOCUS` imports from `use crate::styles::{...}` if they are no longer referenced after cleanup.

- [ ] **Step 4: Final visual smoke test**

Run: `cargo run -- --dry`
Walk through all edge cases:
- Fresh launch (no scan done): "—" + "Scan to check" + Scan button
- After scan with updates: green number + "Updates available"
- Uninstall tile disabled when no packages detected
- Settings gear opens settings
- Profile buttons navigate correctly
- Window resizing doesn't break layout

- [ ] **Step 5: Commit any polish**

```bash
git add -A
git commit -m "chore: polish dashboard home screen, clean up unused code"
```
