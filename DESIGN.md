# Design System

This project uses a dark-first "private media command center" visual system. Interfaces should feel like a personal relay control deck: subdued, functional, ambient, and hyper-usable. The design favors low-noise surfaces that fade into the background and let content—streams, chat, recordings—take center stage.

---

## Stack

- **Framework:** React 19 + Vite+ + TypeScript
- **Styling:** Plain CSS with CSS custom properties (no utility framework)
- **Components:** React components in `web/src/components/`
- **Icons:** `lucide-react`
- **Fonts:** IBM Plex Sans (primary), Noto Sans / system `sans-serif` (fallback)
- **Dark mode:** Theming via CSS custom properties, switched by setting `data-theme` on `<body>` (set imperatively by `TwitchLayout` and `YouTubeLayout`)
- **Utilities:** Plain CSS classes (no `class:` directive, no CSS-in-JS)

---

## Visual Direction

The app is a private relay control center, not a consumer dashboard.

- Use deep ink backgrounds with subtle radial gradients and soft surface overlays.
- Accent colors should be atmospheric: cool blue (`#82aaff`) for Twitch, saturated red (`#ff0033`) for YouTube.
- Cards and panels use gradient backgrounds with low-opacity borders—never flat solid fills.
- Channel cards should feel like dossiers: compact, information-dense, with controls that surface on interaction.
- The watch page is a dark theater: video area dominates, chat is compact and scrollable, overlay controls appear on hover.
- Avoid bright white text, harsh borders, and dense grids. Everything should breathe.

---

## Tokens

All semantic tokens are CSS custom properties defined in `web/src/lib/styles/app.css` inside `:root` and `body[data-theme="youtube"]`. The Twitch/YouTube layout components set `data-theme` on `<body>` on mount; removing the attribute (unmount) falls back to `:root`.

### Base tokens (`:root` — default / Twitch — Tokyo Night Moon)

| Token | Value | Usage |
| --- | --- | --- |
| `--bg` | `#1e2030` | Deepest page background |
| `--bg-soft` | `#222436` | Subtle elevated background |
| `--surface` | `#2f334d` | Card/panel surfaces |
| `--surface-2` | `#3b4261` | Stronger surface (avatars, active states) |
| `--fg` | `#c8d3f5` | Primary text |
| `--muted` | `#a9b8e8` | Secondary/muted text |
| `--accent` | `#82aaff` | Primary action color (blue) |
| `--accent-hover` | `color-mix(in srgb, var(--accent) 85%, white)` | Accent hover state (overridden by derived-token block) |
| `--accent-soft` | `rgba(130, 170, 255, 0.16)` | Accent tint for backgrounds |
| `--accent-border` | `rgba(130, 170, 255, 0.38)` | Accent-tinted border |
| `--accent-2` | `#c099ff` | Secondary accent (purple, for watch quality indicators) |
| `--success` | `#c3e88d` | Live indicators, success messages |
| `--warn` | `#ffc777` | Notice messages, auto-recording states |
| `--danger` | `#ff757f` | Error, delete actions, manual recording active |
| `--border` | `#444a73` | Panel and input borders |
| `--ring` | `rgba(130, 170, 255, 0.45)` | Focus ring |
| `--focus-ring` | `rgba(130, 170, 255, 0.5)` | Stronger focus indicator |

### Derived tokens (`:root` — `color-mix` block)

A second `:root` block in `app.css` re-derives a number of tokens from the base palette. Both themes override these in their respective blocks so the YouTube palette stays consistent throughout:

| Token | Derived From | Usage |
| --- | --- | --- |
| `--border-subtle` | `--border` 65% transparent | Subtle dividers |
| `--border-soft` | `--border` 72% transparent | Default low-contrast borders |
| `--border-medium` | `--border` 75% transparent | Slightly stronger borders |
| `--border-strong` | `--border` 78% transparent | High-contrast borders |
| `--accent-border-hover` | `--accent` 68% white | Hovered accent border |
| `--accent-border-soft` | `--accent` 55% `--border` | Muted accent border |
| `--bg-panel` | `--bg-soft` 95% transparent | Translucent panel background |
| `--bg-elevated` | `--surface` 95% transparent | Elevated panel background |
| `--bg-card` | `--bg-soft` 60% `--surface` | Card gradient background |
| `--bg-surface-soft` | `--surface` 45% transparent | Soft surface overlay |
| `--bg-surface-medium` | `--surface` 75% transparent | Medium surface overlay |
| `--skeleton-mid` | `--surface-2` 70% `--surface` | Skeleton shimmer midpoint |
| `--success-soft` | `--success` 72% white | Success text variant |
| `--surface-subtle` | `--surface-2` 62% transparent | Subtle surface highlight |
| `--live-soon-bg` | `#ff757f` 24% transparent | "Live soon" indicator background |
| `--live-soon-border` | `#ff757f` 56% transparent | "Live soon" indicator border |
| `--live-active-bg` | `#eb0400` 40% transparent | Active recording background |
| `--live-active-border` | `#eb0400` 74% transparent | Active recording border |

### YouTube theme (`body[data-theme="youtube"]`)

| Token | Value | Usage |
| --- | --- | --- |
| `--bg` | `#2a171d` | Dark red-black background |
| `--bg-soft` | `#342029` | Elevated background |
| `--surface` | `#462a35` | Card surfaces |
| `--surface-2` | `#5a3342` | Stronger surfaces |
| `--border` | `#7b3f52` | Panel/input borders |
| `--accent` | `#ff0033` | YouTube red primary |
| `--accent-hover` | `color-mix(in srgb, var(--accent) 85%, white)` | Accent hover (overridden by derived-token block) |
| `--accent-soft` | `rgba(255, 0, 51, 0.16)` | Accent tint |
| `--accent-border` | `rgba(255, 0, 51, 0.35)` | Accent border |
| `--focus-ring` | `rgba(255, 0, 51, 0.5)` | Focus indicator |
| `--success` | `#4caf50` | Success |
| `--danger` | `#ff5252` | Error/delete |
| `--warn` | `#ffb74d` | Warning |
| `--ring` | `rgba(255, 0, 51, 0.35)` | Focus ring |

The YouTube theme also re-derives every token from the derived-tokens block using the new base colors. **`--accent-2` is intentionally not overridden** — the watch page quality indicator stays purple (`#c099ff`) on YouTube.

Both themes use the same token names. Switching is done by setting `data-theme="youtube"` on `<body>` from `YouTubeLayout`; Twitch routes remove the attribute and fall through to `:root`.

---

## Typography

| Token | Font | Usage |
| --- | --- | --- |
| Body | IBM Plex Sans, Noto Sans, system sans-serif | All UI text, controls, forms |

IBM Plex Sans is loaded from Google Fonts in `web/index.html` (weights 400/500/600/700, `display=swap`); the remaining names are declared as fallbacks only.

Guidelines:

- No explicit heading font family — IBM Plex Sans works well for both body and display.
- Channel names are lowercase, bold, weight 600.
- Meta text (source labels, timestamps) uses uppercase, wide letter-spacing (`0.07em–0.16em`), small sizes (`0.68rem–0.74rem`).
- Page titles use `clamp(1.45rem, 4vw, 1.9rem)` for responsive scaling with tight `line-height: 1.1`.
- Video overlay controls use small fonts (`12px–13px`) to stay out of the way.
- Chat messages use `0.9rem` with username-weight differentiation (accent color, weight 600 for the sender).

---

## Core CSS Utilities

Defined in `web/src/lib/styles/app.css` as class-based utilities prefixed with `ui-`:

- `.ui-hide-scrollbar` — Cross-browser scrollbar hiding (`scrollbar-width: none` + WebKit pseudo-elements).
- `.ui-page-shell` — Full-height centered grid layout container with `100dvh`.
- `.ui-page-shell--centered` — Centers content vertically (for auth/login pages).
- `.ui-page-panel` — Gradient card wrapper with border, shadow, and `min(42rem, 100%)` width.
- `.ui-page-panel--wide` — Widens the panel to `min(74rem, 96vw)` (used by the watch page and YouTube video routes).
- `.ui-page-panel--narrow` — Narrows the panel to `min(24rem, 100%)` (used by login forms).
- `.ui-page-header` — Flex header with eyebrow + title + subtle description pattern.
- `.ui-page-eyebrow` — Uppercase, wide-tracked label above page titles.
- `.ui-page-title` — Clamped responsive H1.
- `.ui-page-subtle` — Muted description text below titles.
- `.ui-card` — Card base with gradient background and low-opacity border.
- `.ui-card-interactive` — Hover/focus transitions for clickable cards.
- `.ui-media-row` — Grid layout for avatar/thumbnail + text rows.
- `.ui-media-visual` / `.ui-media-main` / `.ui-media-title` / `.ui-media-meta` — Media item sub-components.
- `.ui-avatar` — Circular avatar with skeleton loading animation (pulse gradient).
- `.ui-thumbnail` — Thumbnail with same skeleton loading behavior.
- `.ui-avatar-fallback` — Centered initial letter for missing images.
- `.ui-nav-chip` — Outline-style navigation button (transparent, bordered, rounded).
- `.ui-ghost-btn` — Transparent button with minimal border.
- `.ui-input` — Dark input field with rounded corners.
- `.ui-section-title` — Consistent section heading.
- `.ui-error` — Error alert block with red background and border.
- `.ui-muted` — Muted text for secondary content.
- `.ui-list` — Grid container with `0.75rem` gap.
- `.ui-form` / `.ui-field` / `.ui-label` — Form layout utilities.
- `.ui-button-primary` — Accent-filled primary action button.
- `.ui-action-row` — Right-aligned action button container.
- `.ui-success-message` / `.ui-success-text` / `.ui-success-subtext` — Success state layout.
- `.ui-alert-success` — Green success alert box.
- `.ui-panel-header--centered` — Centered panel header for auth pages.
- `.ui-chat-composer` — Flex wrapper for the chat input row.
- `.ui-chat-composer-input` — Compact 2.2rem-tall single-line chat input (contenteditable div, not a `<textarea>`).
- `.ui-chat-composer-emote-wrap` — Inline wrapper for inline emotes.
- `.ui-chat-composer-emote` — Inline `<img>` emote sized to `1.35em`.
- `.ui-chat-suggestions` — Absolutely-positioned dropdown above the input (z-index 45).
- `.ui-chat-suggestion-item` — One row in the suggestion dropdown.
- `.ui-chat-emote-preview` / `.ui-chat-emote-preview.visible` — 112×112px fixed-position preview popup with fade/scale animation.

Global body background uses a radial gradient:
```css
background: radial-gradient(
  circle at 20% -10%,
  color-mix(in srgb, var(--surface-2) 88%, black) 0%,
  var(--bg-soft) 45%,
  var(--bg) 100%
);
```

Scrollbars are hidden globally via `scrollbar-width: none`.

---

## Components

### Channel Cards

Channel cards are the primary building block of the home page. They show channel avatar, name, source, stream title/game, and control buttons (watch, auto-record, manual record, remove).

- Grid layout: 74px avatar column + fluid content column.
- Fixed height: `5.5rem`.
- Live channels show a green left-border accent and pulsing status dot on the avatar.
- Control buttons are dark pill icons (`2.35rem` square) with translucent backgrounds.
- The remove button is hidden by default and **reveals on hover** via `width: 0` → `width: var(--ctrl-h)` transition, giving a declutter effect.
- On touch devices, the remove button is always visible.

```tsx
<article className={`channel-card ${status?.live ? 'live' : ''}`}>
  ...
</article>
```

### Buttons

- **Play/Watch buttons**: Accent-filled, dark text (`#1e2030`), pill shape.
- **Icon buttons**: Translucent dark background with semi-transparent border. Hover reveals accent border.
- **Recording buttons**: Danger-tinted by default. Active state fills with solid amber (auto) or red (manual).
- **Navigation chips** (`.ui-nav-chip`): Transparent, bordered, rounded pills for navigation.
- **Ghost buttons** (`.ui-ghost-btn`): Minimal border, used for secondary actions.

### Inputs

Inputs use `.ui-input`: dark background (`rgba(8, 12, 19, 0.9)`), rounded (`0.6rem`), semi-transparent border. Focus states get accent-tinted outline + `box-shadow` ring.

Chat input in the watch page is a single-line `contenteditable` div (not a `<textarea>`), styled as a compact input bar with accent focus ring.

### Empty States

Empty states are rendered by the `EmptyState` component (`web/src/components/ui/empty-state.tsx`) with class names `.empty-state`, `.empty-icon`, `.empty-title`, `.empty-description`, `.empty-action`. Used when no channels are configured, no recordings exist, or the live-only filter returns nothing.

### Skeleton Loading

Avatars and thumbnails animate a shimmer pulse gradient (`.ui-avatar`, `.ui-thumbnail`) until the `src` attribute is set, at which point the animation stops. Higher-level lists are rendered by dedicated React components in `web/src/components/ui/`:

- `SkeletonMediaList` — 8 default rows with circular avatars (used for channel lists).
- `SkeletonVideoList` — 5 default rows with 16:9 thumbnails (used for YouTube video lists).
- `SkeletonRecordingList` — 3 default sections of 3 items each (used for recordings pages).
- `SkeletonThumbnail` — Standalone thumbnail with configurable width / height / aspect ratio / border radius.
- `SkeletonText` — Stack of `skeleton-line` rows with configurable widths.

### Confirm Dialogs

Reusable `ConfirmDialog` component for destructive actions (remove channel, delete recording, merge recording). Translucent backdrop (`.modal-overlay`) wraps a centered panel (`.modal`) containing a content area (`.modal-content`) and a right-aligned action row (`.modal-actions`) with cancel (`.ui-ghost-btn`) and confirm buttons. The confirm button uses a `.primary` or `.danger` modifier class. Enter/exit transitions use `.entering` / `.exiting` modifier classes.

### Emote Picker

A popup panel attached to the chat input:
- Search input at the top.
- Emotes grouped by source (Channel, 7TV, BTTV).
- 6-column grid, 44px touch-friendly buttons.
- Emote images max 30px, auto-sized.
- `max-height: min(52vh, 420px)` with internal scroll.

### Video Player Overlay

Watch page video controls (quality selector, go-live button) overlay the video with a gradient fade at the top. Hidden by default (`opacity: 0`, `pointer-events: none`), revealed on `.video-shell:hover` and `.overlay-controls:focus-within` for keyboard users. Always visible on touch devices via `@media (hover: none)`.

---

## Layout

### Page Shell

All routes use the page shell pattern:

```tsx
<div className="ui-page-shell">
  <div className="ui-page-panel">
    <div className="ui-page-header">
      <p className="ui-page-eyebrow">...</p>
      <h1 className="ui-page-title">...</h1>
      <p className="ui-page-subtle">...</p>
    </div>
    {/* content */}
  </div>
</div>
```

The panel is centered and constrained to `min(42rem, 100%)` on desktop, full width on mobile.

### Watch Page Layout

The watch page uses a custom two-column grid:

```css
grid-template-columns: minmax(0, 1fr) clamp(280px, 19vw, 380px);
```

- Left: Video player (full height, `object-fit: contain`, black background).
- Right: Chat panel (fixed width column, independently scrollable).
- On mobile (`max-width: 900px`): Stacks vertically, video uses `aspect-ratio: 16/9`.

### YouTube Shell

YouTube routes use the `YouTubeShell` component, composed of three pieces:

- `RelayHeader` — header with "Private Deck" eyebrow label, the active relay title ("YouTube Relay"), and a left-right arrow button that navigates to the other relay route. The toggle's `aria-label` / tooltip is "Switch to Twitch Relay" or "Switch to YouTube Relay" depending on context.
- `YouTubeNavTabs` — navigation tabs for **Subscriptions**, **Recent**, and **Playlists**.
- A content `<section>` rendering the child route.

`TwitchLayout` mirrors the same structure with a Twitch-flavored title and a YouTube-relay toggle.

### Responsive

Breakpoints used across the app:

| Breakpoint | Used for |
| --- | --- |
| `max-width: 600px` | Mobile small (channel cards, header collapse, mobile recordings) |
| `max-width: 640px` | Twitch recording player responsive layout |
| `max-width: 700px` | Mid-size collapse for shared surfaces |
| `max-width: 768px` and `min-width: 601px` | Header collapses to hamburger menu |
| `max-width: 900px` | Watch page stacks vertically |
| `1000px–1400px × 600px–800px` (landscape) | TV landscape (e.g. Xbox Edge) — reduces panel padding for living-room browsers |
| `100px–1400px × 600px–800px` (landscape) | Permissive TV-landscape variant scoped to the YouTube watch page |

Touch devices (matched with `@media (hover: none)`) get always-visible remove buttons and video overlay controls (no hover-only reveals). Scrollbars are hidden globally.

---

## Interaction

- **Hover lift**: Channel cards lift `-1px` on hover.
- **Hover reveal**: Remove button expands from 0-width to full size with `transition: width 0.2s ease`.
- **Focus**: `focus-visible` uses `box-shadow: 0 0 0 3px var(--focus-ring)`.
- **Disabled**: `opacity: 0.6–0.65`, `cursor: not-allowed`.
- **Transitions**: Border-color and background-color use `0.15s–0.2s ease` transitions for smooth state changes.
- **Skeleton pulse**: Shimmer animation at `1.5s ease-in-out infinite`.
- **Live dot pulse**: Green status dot fades between `opacity: 1` and `opacity: 0.4`.
- **Spinner**: `.spinning` class with `spin 0.8s linear infinite`.
- **Reduced motion**: Skeleton animations are disabled when `prefers-reduced-motion: reduce`.

---

## Accessibility

- Visible `focus-visible` rings on all interactive controls with `--focus-ring` color.
- Touch-friendly button sizes (minimum `44px` height for emote items).
- Always-visible controls on touch devices (no hover-only reveals where it matters).
- Text contrast maintained against dark backgrounds (light text on dark surfaces).
- Form inputs have visible focus indicators.
- Controls use `aria-label` for icon-only buttons.
- Contenteditable chat input uses `data-placeholder` attribute for placeholder text.
- Reduced motion media query respected for loading animations.
