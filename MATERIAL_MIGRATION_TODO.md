# Material Design Migration TODO

This document is the persistent handoff for the Material Design conversion. It is written so a future AI agent can continue safely after context compaction.

## Current Branch

- Branch: `feature/material-design-migration`
- Remote tracking: `origin/feature/material-design-migration`
- Important: this branch has been pushed. Re-check local worktree state before continuing.
- Before continuing, run `git status --short --branch` and inspect any local changes. Do not revert user or agent changes unless explicitly instructed.

## Repository Rules To Preserve

- Follow `AGENTS.md`.
- Frontend commands must run from `web/`.
- Do not add lint suppression comments.
- Do not add `#[allow(...)]` attributes.
- Fix strict lint issues at the root cause.
- Use subagents where useful for search, review, docs, and focused implementation support.
- Run verification after each meaningful phase.
- Keep the dark-first relay design system from `DESIGN.md`; Material should be adapted to the app, not applied as generic default MUI chrome.
- Version bumps are required for bug fixes/small improvements that constitute a patch release, but subagents must not bump versions.

## Product And Design Decisions Already Made

- Use MUI Material as the component and theme foundation.
- Use `@mui/icons-material`; do not keep icon migration deferred indefinitely.
- Use Twitch and YouTube brand colors inside Material themes.
- Preserve a dense, dark, media-command-center feel from `DESIGN.md`.
- Use dense `Card`/`Paper` rows for channel and recording surfaces.
- Replace the old chat composer directly; do not keep a feature flag fallback.
- Use Lexical for the strict-internals chat composer rewrite.
- Use inline graphical emotes while typing.
- Include keyboard-only undo/redo in the composer.
- Keep outbound chat submission plain text for `/api/chat/send`.
- Prefer clean MUI architecture over layering MUI on top of the old CSS forever.

## Completed Phases

### Done: Phase 0 - MUI Foundation

- Commit: `eab13074`
- Added MUI dependencies:
  - `@mui/material`
  - `@emotion/react`
  - `@emotion/styled`
  - `@mui/icons-material`
- Added route-driven theme infrastructure under `web/src/lib/theme/`.
- Added Twitch theme in `web/src/lib/theme/twitch-theme.ts`.
- Added YouTube theme in `web/src/lib/theme/youtube-theme.ts`.
- Added theme context in `web/src/lib/theme/theme-context.tsx`.
- Added route theme derivation in `web/src/lib/theme/use-route-theme.ts`.
- Wrapped the app with MUI `ThemeProvider` and `CssBaseline` in `web/src/main.tsx`.
- Removed `document.body.dataset.theme` mutation from:
  - `web/src/components/layout/twitch-layout.tsx`
  - `web/src/components/layout/you-tube-layout.tsx`
- Verification at that point passed:
  - `pnpm run typecheck`
  - `pnpm run lint`
  - `pnpm run test`
  - `pnpm run build`

### Done: Phase 1 - Lexical Dependencies

- Commit: `31c4624`
- Added Lexical dependencies:
  - `lexical`
  - `@lexical/react`
  - `@lexical/plain-text`
  - `@lexical/utils`
  - `@lexical/selection`
  - `@lexical/history`

### Done: Phase 2 - Direct Composer Replacement

- Commit: `3317cb08`
- Added Lexical composer implementation under `web/src/components/lexical-chat-composer/`.
- Added custom inline emote node.
- Added `:` emote autocomplete.
- Added keyboard navigation for autocomplete.
- Added single-line enforcement.
- Added paste normalization.
- Added 500-character max-length handling.
- Added keyboard-only undo/redo via Lexical history.
- Replaced old `ChatComposer` usage in `web/src/components/watch/chat.tsx`.
- Kept existing `EmotePicker` and routed picker insertion through the Lexical composer ref.
- Removed legacy composer files:
  - `web/src/components/watch/chat-composer.tsx`
  - `web/src/hooks/watch/use-chat-composer.ts`
  - `web/src/hooks/watch/chat-composer-content.ts`
  - `web/src/hooks/watch/chat-composer-cursor.ts`
  - `web/src/hooks/watch/chat-composer-cursor-position.ts`
  - `web/src/hooks/watch/chat-composer-emotes.ts`
  - `web/src/hooks/watch/chat-composer-helpers.ts`
  - `web/src/hooks/watch/chat-composer-insert.ts`
  - `web/src/hooks/watch/chat-composer-keyboard.ts`
  - `web/src/hooks/watch/chat-composer-preview.ts`
  - `web/src/hooks/watch/chat-composer-preview-state.ts`
  - `web/src/hooks/watch/chat-composer-suggestions.ts`
- Updated exports in:
  - `web/src/components/watch/index.ts`
  - `web/src/hooks/index.ts`

## Current Verification Status

- `pnpm run verify` from `web/`: passed after composer cleanup.
- Verification includes:
  - `pnpm run fmt:check`
  - `pnpm run lint`
  - `pnpm run typecheck`
  - `vp check`
  - `pnpm run test`
  - `pnpm run build`
- Build currently emits a Vite chunk-size warning because the MUI/Lexical bundle is larger, but the build succeeds.

## Done: Phase 2A - Composer Lint Cleanup

### Goal

Make the new Lexical composer pass strict frontend lint while preserving behavior. Do not use suppression comments.

Status: complete.

### Files In Scope

- `web/src/components/lexical-chat-composer/lexical-chat-composer.tsx`
- `web/src/components/lexical-chat-composer/nodes/emote-node.ts`
- `web/src/components/lexical-chat-composer/plugins/single-line-plugin.tsx`
- `web/src/components/lexical-chat-composer/plugins/emote-autocomplete-plugin.tsx`
- `web/src/components/watch/chat.tsx`

### Completed Lint Work

- Renamed remaining PascalCase files to kebab-case.
- Updated all imports after renames.
- Converted declarations and callbacks to satisfy strict lint rules.
- Added explicit return types to cleanup functions and callbacks.
- Replaced magic numbers with named constants.
- Replaced short identifiers with descriptive names.
- Sorted object keys in MUI `sx`, inline `style`, and plain object literals.
- Avoided unsafe type assertions with `instanceof Node` checks.
- Avoided confusing void expressions in JSX callbacks.
- Added no lint-disable comments.
- Preserved composer behavior.

### Composer Acceptance Criteria

- `Chat` uses `LexicalChatComposer` directly.
- `EmotePicker` inserts into the Lexical editor through the composer ref.
- Typing text and pressing Enter submits plain text.
- Inline emotes render as images while typing.
- `:` autocomplete opens and inserts emotes.
- ArrowUp/ArrowDown navigate autocomplete.
- Enter/Tab accept autocomplete when suggestions are open.
- Escape closes autocomplete.
- Paste normalizes line breaks into spaces.
- The editor remains single-line.
- Max length is enforced at 500 serialized text characters.
- Ctrl/Cmd+Z and redo shortcuts work through Lexical history.
- Disabled/sending state prevents edits and submit.

### Verification Required After Cleanup

Run from `web/`:

```bash
pnpm run typecheck
pnpm run lint
pnpm run test
pnpm run build
```

Then run the full project frontend verify if feasible:

```bash
pnpm run verify
```

## Remaining Material Conversion Phases

### Done: Phase 3 - Shared MUI Primitives

- Commit: (pending)
- Converted all UI primitives to MUI:
  - `confirm-dialog.tsx`: MUI `Dialog`, `DialogContent`, `DialogActions`, `Button`
  - `error-state.tsx`: MUI `Alert`, `Button`, `CircularProgress`
  - `empty-state.tsx`: MUI `Paper`, `Box`, `Typography` with `lucide-react` icons
  - `media-row.tsx`: MUI `Card`, `CardActionArea`, `Avatar`, `Typography`
  - `loaded-fade.tsx`: MUI `Fade`
  - `skeleton-text.tsx`: MUI `Skeleton` variant="text"
  - `skeleton-thumbnail.tsx`: MUI `Skeleton` variant="rectangular"
  - `skeleton-media-list.tsx`: MUI `Stack`, `Box`, `Skeleton`
  - `skeleton-video-list.tsx`: MUI `Stack`, `Box`, `Skeleton`
  - `skeleton-recording-list.tsx`: MUI `Stack`, `Box`, `Skeleton`
- Replaced custom CSS classes with MUI components throughout.
- Kept `lucide-react` icons for EmptyState (not yet migrated to `@mui/icons-material`).
- Verification passes:
  - `pnpm run verify` (fmt, lint, typecheck, test, build)

### Done: Phase 4 - App Shell, Header, And Navigation

- Commit: (pending)
- Converted header chrome to MUI components:
  - `relay-header.tsx`: MUI `AppBar`, `Toolbar`, `Typography`, `IconButton`, `Box`
  - `app-header.tsx`: MUI `AppBar`, `Toolbar`, `Typography`, `Chip`, `Box`
  - `app-header-actions.tsx`: MUI `Menu`, `MenuItem`, `IconButton`, `Button`, `Box` (desktop + mobile responsive)
  - `nav-tabs.tsx`: MUI `Tabs`, `Tab` (YouTube navigation)
  - `you-tube-nav.tsx`: MUI `Tabs`, `Tab`
- Converted layout files:
  - `twitch-layout.tsx`: MUI `Box` with focus management preserved
  - `you-tube-layout.tsx`: MUI `Box` with focus management preserved
  - `app-version.tsx`: MUI `Typography`
  - `you-tube-shell.tsx`: MUI `Box`
- Replaced custom dropdown menu with MUI `Menu` component (mobile responsive)
- Replaced status dots with MUI `Chip` components
- Preserved all focus management and navigation behavior
- Verification passes:
  - `pnpm run verify` (fmt, lint, typecheck, test, build)

### Done: Phase 5 - Twitch Channel Management Surface

- Commit: (pending)
- Converted Twitch channel management to MUI:
  - `twitch-panel.tsx`: MUI `Paper`
  - `twitch-channels-view.tsx`: MUI `Stack`, `Typography`, `Switch`, `FormControlLabel`, `Button`
  - `channel-card.tsx`: MUI `Card`, `CardContent`, `Avatar`, `IconButton`, `Chip`, `Typography`, `Box`
  - `add-channel-form.tsx`: MUI `Box`, `TextField`, `Button`
  - `auth-panel.tsx`: MUI `Box`, `TextField`, `Button`, `Paper`, `Typography`
  - `twitch-home-page.tsx`: MUI `Alert` for error messages
- Replaced custom CSS classes throughout with MUI components
- Kept `lucide-react` icons in channel-card (not yet migrated to `@mui/icons-material`)
- Channel cards now use dense MUI `Card` layout with `Avatar` and `Chip` for live indicator
- Added responsive `Stack` layouts for header and actions
- Verification passes:
  - `pnpm run verify` (fmt, lint, typecheck, test, build)

### Done: Phase 6 - Twitch Recordings Surface

- Commit: (pending)
- Converted Twitch recordings surface to MUI:
  - `recordings-overview.tsx`: MUI `Box`, `Stack`, `Typography`, `Button`
  - `recording-filters.tsx`: MUI `FormControl`, `InputLabel`, `Select`, `MenuItem`
  - `recording-actions.tsx`: MUI `Box`, `IconButton`, `Tooltip`, `CircularProgress`
  - `recording-badges.tsx`: MUI `Box`, `Chip`
  - `active-recording-section.tsx`: MUI `Paper`, `Typography`, `List`, `ListItem`
  - `completed-recordings-section.tsx`: MUI `Paper`, `Typography`, `List`
  - `completed-recording-row.tsx`: MUI `ListItem`, `ListItemText`, `Box`
  - `incomplete-recordings-section.tsx`: MUI `Paper`, `Typography`, `List`
  - `incomplete-recording-row.tsx`: MUI `ListItem`, `ListItemText`, `IconButton`, `CircularProgress`
  - `incomplete-section-header.tsx`: MUI `Box`, `Typography`, `Button`, `CircularProgress`
- Replaced custom CSS classes with MUI components throughout
- Kept `lucide-react` icons in recording-actions and incomplete-recording-row
- All sections now use dense MUI `Paper` and `List` layouts
- Verification passes:
  - `pnpm run verify` (fmt, lint, typecheck, test, build)

### Pending: Phase 7 - YouTube Surfaces

Files likely in scope:

- `web/src/pages/you-tube-home-page.tsx`
- `web/src/pages/you-tube-recent-page.tsx`
- `web/src/pages/you-tube-playlists-page.tsx`
- `web/src/pages/you-tube-playlist-page.tsx`
- `web/src/pages/you-tube-channel-page.tsx`
- `web/src/components/youtube/you-tube-shell.tsx`
- `web/src/components/youtube/you-tube-nav.tsx`
- `web/src/components/youtube/you-tube-video-row.tsx`

Targets:

- Convert YouTube list/detail surfaces to MUI `Container`, `Paper`, `Card`, `CardActionArea`, `Tabs`, `Typography`, `Stack`.
- Preserve YouTube red brand theme.
- Replace back buttons and nav chips consistently.

Acceptance criteria:

- Home, recent, playlists, playlist detail, and channel routes render correctly.
- Active tabs are correct.
- Long titles/descriptions truncate cleanly.
- Back navigation works.

### Pending: Phase 8 - Watch And Player UI

Files likely in scope:

- `web/src/pages/watch-page.tsx`
- `web/src/components/watch/video-player.tsx`
- `web/src/components/watch/chat.tsx`
- `web/src/components/watch/emote-picker.tsx`
- `web/src/pages/you-tube-watch-page.tsx`
- `web/src/pages/you-tube-watch/player-content.tsx`

Targets:

- Convert watch page shell/actions to MUI.
- Convert video overlay buttons/quality menu to MUI buttons/menus where safe.
- Convert chat panel shell to MUI `Paper`/`Stack`/`List` patterns.
- Ensure popup z-index behavior works over video.

Acceptance criteria:

- Playback starts correctly.
- Quality menu works mouse, keyboard, and touch.
- Chat composer still works after any surrounding layout changes.
- Error states remain visible.

### Pending: Phase 9 - Edge Routes And Cleanup

Files likely in scope:

- `web/src/pages/qr-login-page.tsx`
- `web/src/pages/index-redirect.tsx`
- `web/src/app.tsx`
- `web/src/lib/styles/app.css`

Targets:

- Convert QR/login and 404/redirect edge surfaces.
- Remove obsolete CSS from `app.css` after components no longer rely on it.
- Remove stale legacy class names and bridge code.

Acceptance criteria:

- QR login still works.
- 404 route still works.
- No stale imports or dead exports.
- `pnpm run verify` passes.

## Final Definition Of Done

- `pnpm run verify` passes from `web/`.
- No lint suppressions were added.
- No `#[allow(...)]` attributes were added.
- All touched UI follows `DESIGN.md` and the Material-with-brand-colors direction.
- Twitch and YouTube routes keep correct theme on hard refresh and client navigation.
- Composer supports required behavior with no regressions.
- Legacy CSS and utility classes are removed when no longer used.
- Branch is pushed after clean verification.
