# Design System

This project uses a dark-first "private media command center" visual system. Interfaces should feel like a personal relay control deck: subdued, functional, ambient, and hyper-usable. The design favors low-noise surfaces that fade into the background and let content—streams, chat, recordings—take center stage.

---

## Stack

- **Framework:** SvelteKit (SSR disabled, SPA mode) + TypeScript
- **Styling:** Plain CSS with CSS custom properties (no utility framework)
- **Components:** Svelte components in `web/src/lib/components/`
- **Icons:** Lucide Svelte (`lucide-svelte`)
- **Fonts:** Space Grotesk (primary), IBM Plex Sans / Noto Sans (fallback)
- **Dark mode:** Theming via CSS custom properties, switched with `data-theme` attribute on `<body>`
- **Utilities:** `class:` directive (Svelte built-in), no runtime CSS-in-JS

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

All semantic tokens are CSS custom properties defined in `web/src/routes/+layout.svelte` inside `:root` and `:global(body[data-theme="youtube"])`.

### Default Theme (Twitch — Tokyo Night Moon)

| Token | Value | Usage |
| --- | --- | --- |
| `--bg` | `#1e2030` | Deepest page background |
| `--bg-soft` | `#222436` | Subtle elevated background |
| `--surface` | `#2f334d` | Card/panel surfaces |
| `--surface-2` | `#3b4261` | Stronger surface (avatars, active states) |
| `--fg` | `#c8d3f5` | Primary text |
| `--muted` | `#a9b8e8` | Secondary/muted text |
| `--accent` | `#82aaff` | Primary action color (blue) |
| `--accent-hover` | `#a8c5ff` | Accent hover state |
| `--accent-soft` | `rgba(130, 170, 255, 0.16)` | Accent tint for backgrounds |
| `--accent-border` | `rgba(130, 170, 255, 0.38)` | Accent-tinted border |
| `--accent-2` | `#c099ff` | Secondary accent (purple, for watch quality indicators) |
| `--success` | `#c3e88d` | Live indicators, success messages |
| `--warn` | `#ffc777` | Notice messages, auto-recording states |
| `--danger` | `#ff757f` | Error, delete actions, manual recording active |
| `--border` | `#444a73` | Panel and input borders |
| `--ring` | `rgba(130, 170, 255, 0.45)` | Focus ring |
| `--focus-ring` | `rgba(130, 170, 255, 0.5)` | Stronger focus indicator |

### YouTube Theme (`data-theme="youtube"`)

| Token | Value | Usage |
| --- | --- | --- |
| `--bg` | `#2a171d` | Dark red-black background |
| `--bg-soft` | `#342029` | Elevated background |
| `--surface` | `#462a35` | Card surfaces |
| `--surface-2` | `#5a3342` | Stronger surfaces |
| `--border` | `#7b3f52` | Panel/input borders |
| `--accent` | `#ff0033` | YouTube red primary |
| `--accent-hover` | `#cc0029` | Accent hover |
| `--accent-soft` | `rgba(255, 0, 51, 0.16)` | Accent tint |
| `--accent-border` | `rgba(255, 0, 51, 0.35)` | Accent border |
| `--focus-ring` | `rgba(255, 0, 51, 0.5)` | Focus indicator |
| `--success` | `#4caf50` | Success |
| `--danger` | `#ff5252` | Error/delete |
| `--warn` | `#ffb74d` | Warning |
| `--ring` | `rgba(255, 0, 51, 0.35)` | Focus ring |

Both themes use the same token names. Switching is done by setting `data-theme="youtube"` on `<body>` for YouTube routes; Twitch routes are the default.

---

## Typography

| Token | Font | Usage |
| --- | --- | --- |
| Body | Space Grotesk, IBM Plex Sans, Noto Sans | All UI text, controls, forms |

Guidelines:

- No explicit heading font family—Space Grotesk works well for both body and display.
- Channel names are lowercase, bold, weight 600.
- Meta text (source labels, timestamps) uses uppercase, wide letter-spacing (`0.07em–0.16em`), small sizes (`0.68rem–0.74rem`).
- Page titles use `clamp(1.45rem, 4vw, 1.9rem)` for responsive scaling with tight `line-height: 1.1`.
- Video overlay controls use small fonts (`12px–13px`) to stay out of the way.
- Chat messages use `0.9rem` with username-weight differentiation (accent color, weight 600 for the sender).

---

## Core CSS Utilities

Defined in `web/src/lib/styles/app.css` as class-based utilities prefixed with `ui-`:

- `.ui-page-shell` — Full-height centered grid layout container with `100dvh`.
- `.ui-page-shell--centered` — Centers content vertically (for auth/login pages).
- `.ui-page-panel` — Gradient card wrapper with border, shadow, and `min(42rem, 100%)` width.
  - `--wide` (74rem) for video playback pages.
  - `--narrow` (24rem) for login forms.
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

```svelte
<article class="channel-card" class:live={status?.live}>
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

Empty states (`.ui-empty-state`-style, component-specific) show a centered message with title, description, and optional action. Used when no channels are configured, no recordings exist, or live-only filter returns nothing.

### Skeleton Loading

Avatars and thumbnails animate a shimmer pulse gradient (`.ui-avatar`, `.ui-thumbnail`) until the `[src]` attribute is set, at which point the animation stops. Media lists use dedicated skeleton components (`SkeletonMediaList`, `SkeletonVideoList`, `SkeletonRecordingList`).

### Confirm Dialogs

Reusable `ConfirmDialog` component for destructive actions (remove channel, delete recording, merge recording). Overlay with translucent backdrop, centered panel with title, message, cancel/confirm buttons.

### Emote Picker

A popup panel attached to the chat input:
- Search input at the top.
- Emotes grouped by source (Channel, 7TV, BTTV).
- 6-column grid, 44px touch-friendly buttons.
- Emote images max 30px, auto-sized.
- `max-height: min(52vh, 420px)` with internal scroll.

### Video Player Overlay

Watch page video controls (quality selector, go-live button) overlay the video with a gradient fade at the top. Hidden by default (`opacity: 0`), revealed on `.watch-video-shell:hover`. Always visible on touch devices.

---

## Layout

### Page Shell

All routes use the page shell pattern:

```svelte
<div class="ui-page-shell">
  <div class="ui-page-panel">
    <div class="ui-page-header">
      <p class="ui-page-eyebrow">...</p>
      <h1 class="ui-page-title">...</h1>
      <p class="ui-page-subtle">...</p>
    </div>
    <!-- content -->
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

YouTube routes use `YouTubeShell` component with:
- "Private Deck" eyebrow label.
- Switch button to jump between Twitch Relay and YouTube Relay.
- Navigation tabs: Subscriptions, Recent, Playlists.
- Content area for child routes.

### Responsive

- Mobile breakpoint at `600px` for channel cards, `900px` for watch page.
- Touch devices get always-visible remove buttons (no hover reveal).
- TV landscape media query (`1000px–1400px × 600px–800px`) reduces padding for living-room browsers.
- Scrollbars hidden globally.

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
