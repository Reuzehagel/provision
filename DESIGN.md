# Design System — Provision

A shadcn/Tailwind-inspired dark theme design system for the Provision app.

---

## Color Palette

Based on Tailwind's **zinc** scale for neutrals, with semantic accent colors.

### Neutrals (zinc)

| Token            | Hex       | RGB (0–1)             | Usage                           |
| ---------------- | --------- | --------------------- | ------------------------------- |
| `BG`             | `#09090b` | `0.035, 0.035, 0.043` | App background                  |
| `CARD`           | `#18181b` | `0.094, 0.094, 0.106` | Cards, surfaces                 |
| `CARD_HOVER`     | `#27272a` | `0.153, 0.153, 0.165` | Card hover state                |
| `BORDER`         | `#27272a` | `0.153, 0.153, 0.165` | Default borders                 |
| `BORDER_FOCUS`   | `#3f3f46` | `0.247, 0.247, 0.275` | Focused/hover borders           |
| `INPUT`          | `#27272a` | `0.153, 0.153, 0.165` | Input field backgrounds         |
| `MUTED`          | `#71717a` | `0.443, 0.443, 0.478` | Muted/secondary text, icons     |
| `MUTED_FG`       | `#a1a1aa` | `0.631, 0.631, 0.667` | Descriptions, secondary content |
| `TEXT`           | `#fafafa` | `0.980, 0.980, 0.980` | Primary text                    |
| `TEXT_SECONDARY` | `#a1a1aa` | `0.631, 0.631, 0.667` | Subtitles, descriptions         |

### Accent Colors

| Token           | Hex       | RGB (0–1)             | Usage                          |
| --------------- | --------- | --------------------- | ------------------------------ |
| `PRIMARY`       | `#3b82f6` | `0.231, 0.510, 0.965` | Primary buttons, active states |
| `PRIMARY_HOVER` | `#2563eb` | `0.145, 0.388, 0.922` | Primary button hover           |
| `SUCCESS`       | `#10b981` | `0.063, 0.725, 0.506` | Success states, "installed"    |
| `SUCCESS_MUTED` | `#065f46` | `0.024, 0.373, 0.275` | Success badge background       |
| `DANGER`        | `#ef4444` | `0.937, 0.267, 0.267` | Errors, cancel, destructive    |
| `DANGER_HOVER`  | `#dc2626` | `0.863, 0.149, 0.149` | Danger button hover            |
| `WARNING`       | `#f59e0b` | `0.961, 0.620, 0.043` | Warnings, "already installed"  |
| `INFO`          | `#3b82f6` | `0.231, 0.510, 0.965` | In-progress, informational     |

---

## Typography

| Element          | Size | Weight  | Color                        |
| ---------------- | ---- | ------- | ---------------------------- |
| App title        | 28px | Default | `TEXT`                       |
| Screen heading   | 18px | Default | `TEXT`                       |
| Subtitle         | 12px | Default | `TEXT_SECONDARY`             |
| Card title       | 14px | Default | `TEXT`                       |
| Card description | 12px | Default | `MUTED_FG`                   |
| Body / labels    | 13px | Default | `TEXT`                       |
| Small / caption  | 11px | Default | `MUTED`                      |
| Category label   | 10px | Default | `MUTED` (uppercase, tracked) |
| Terminal text    | 11px | Mono    | `MUTED_FG`                   |

---

## Spacing

Consistent spacing scale (multiples of 4):

| Token | Value | Usage                              |
| ----- | ----- | ---------------------------------- |
| `xs`  | 4px   | Tight inline gaps                  |
| `sm`  | 8px   | Within groups, icon-to-label       |
| `md`  | 12px  | Between related elements           |
| `lg`  | 16px  | Between sections, card padding     |
| `xl`  | 24px  | Between major sections             |
| `2xl` | 32px  | Screen padding (all sides)         |
| `3xl` | 40px  | Was 40 before — keeping for compat |

**Screen padding**: 24px all sides
**Card padding**: 14px all sides
**Between cards**: 8px gap
**Between screen sections**: 16px

---

## Border Radius

| Element      | Radius |
| ------------ | ------ |
| Cards        | 8px    |
| Buttons      | 6px    |
| Badges       | 4px    |
| Input fields | 6px    |
| Terminal box | 6px    |
| Progress bar | 4px    |

_(Tighter than current 12px/8px — shadcn uses subtle rounding)_

---

## Components

### Profile Cards (Home Screen)

- **All 4 cards in a 2×2 grid, equal size** using `Length::Fill` (not fixed 340px)
- Each card has: icon (20px) → title (16px) → description (13px)
- Background: `CARD`, border: 1px `BORDER`
- Hover: bg `CARD_HOVER`, border `BORDER_FOCUS`
- Padding: 20px
- Gap between cards: 12px

### Update Card (Home Screen)

- **Same width as the 2×2 grid** — sits in the same container, takes full width
- Same card style as profile cards
- Row layout: icon + [title, description]

### Back Button

- **Icon only**: chevron-left icon (no "< Back" text)
- Ghost style: transparent bg, `MUTED_FG` icon color
- Hover: bg `CARD`, icon `TEXT`
- Size: 28×28px hit target
- Radius: 6px

### Primary Button (Continue, Install, Upgrade)

- Background: `PRIMARY`, text: white
- Hover: `PRIMARY_HOVER`
- Disabled: bg `CARD`, text `MUTED`
- Padding: 8px 20px
- Radius: 6px
- Font size: 14px

### Cancel / Danger Button

- Background: transparent, border 1px `BORDER`, text `DANGER`
- Hover: bg `DANGER` with 10% opacity, border `DANGER`
- Disabled: text `MUTED`, border `BORDER`
- Padding: 8px 20px
- Radius: 6px

### Secondary / Ghost Button

- Background: transparent
- Text: `MUTED_FG`
- Hover: bg `CARD`, text `TEXT`

### Checkbox

- Size: 14×14px
- Checked: `PRIMARY` fill with white check
- Unchecked: `BORDER` border, transparent fill

### Search Input

- Background: `INPUT`
- Border: 1px `BORDER`
- Focus border: `BORDER_FOCUS`
- Placeholder: `MUTED`
- Text: `TEXT`
- Padding: 8px 12px
- Radius: 6px
- Size: 14px

### Badge ("Installed")

- Background: `SUCCESS_MUTED` (dark emerald)
- Text: `SUCCESS`
- Border: 1px with SUCCESS at ~30% opacity
- Padding: 1px 6px
- Radius: 4px
- Font size: 10px

### Terminal Box

- Background: `#0a0a0a` (nearly black)
- Border: 1px `BORDER`
- Radius: 6px
- Text: `MUTED_FG`, monospace, 12px
- Padding: 12px

### Progress Bar

- Track: `CARD`
- Fill: `PRIMARY`
- Height: default (iced doesn't support custom height)
- Radius: 4px

### Screen Header

- Row: [back_button, heading]
- Back button: ghost icon button (chevron-left)
- Heading: 24px `TEXT`
- Gap: 12px, vertically centered

### Footer

- Row: [left_content, spacer, right_buttons]
- Separated from content by top border or visual space
- Vertically centered
- Consistent padding with screen

---

## Icons

Switch from raw hex codepoints (`iced_fonts`) to `lucide-icons` crate with `iced` feature.
Named constants via `Icon` enum: `Icon::ArrowLeft`, `Icon::ChevronLeft`, `Icon::Package`, etc.

### Icon Sizes

| Context       | Size |
| ------------- | ---- |
| Profile card  | 20px |
| Back button   | 16px |
| Status icons  | 14px |
| Inline badges | 12px |

---

## Layout Patterns

### Screen Structure

```
┌──────────────────────────────────────┐
│  padding: 32px                       │
│  ┌──────────────────────────────────┐│
│  │ [←] Screen Title                 ││
│  │                                  ││
│  │ Subtitle text                    ││
│  │                                  ││
│  │ ┌──────────────────────────────┐ ││
│  │ │                              │ ││
│  │ │  Scrollable content area     │ ││
│  │ │                              │ ││
│  │ └──────────────────────────────┘ ││
│  │                                  ││
│  │ Footer: [info]        [buttons]  ││
│  └──────────────────────────────────┘│
└──────────────────────────────────────┘
```

### Home Screen (Profile Select)

```
┌──────────────────────────────────────┐
│           Provision                  │
│    Choose a profile to get started   │
│     153 packages detected            │
│                                      │
│  ┌──────────────┐ ┌──────────────┐   │
│  │ Personal     │ │ Work         │   │
│  │ Browsers,    │ │ Dev tools,   │   │
│  │ media, ...   │ │ comms, ...   │   │
│  └──────────────┘ └──────────────┘   │
│  ┌──────────────┐ ┌──────────────┐   │
│  │ Homelab      │ │ Manual       │   │
│  │ Server util, │ │ Start from   │   │
│  │ containers   │ │ scratch      │   │
│  └──────────────┘ └──────────────┘   │
│                                      │
│  ┌──────────────────────────────┐    │
│  │ Update                       │    │
│  │ Check for outdated packages  │    │
│  └──────────────────────────────┘    │
└──────────────────────────────────────┘
```

All 4 profile cards: same size, all have descriptions.
Update card: same total width as the 2×2 grid.

---

## Key Changes from Current

1. **Colors**: Replace all hardcoded RGB values with zinc-based palette
2. **Back button**: Icon-only (chevron-left), ghost style — no "< Back" text
3. **Card sizing**: `Length::Fill` instead of fixed 340px — responsive, equal
4. **Update card**: Constrained to same width as profile grid (via shared container)
5. **Border radius**: Tighter (8px cards, 6px buttons vs current 12px/8px)
6. **Cancel button**: Outlined style instead of filled red — less aggressive
7. **Spacing**: Standardized scale, consistent across all screens
8. **Icons**: Switch to `lucide-icons` crate for named constants
9. **Typography**: Tighter size scale, more hierarchy
10. **Profile card icons**: 20px (down from 32px) — less dominant
