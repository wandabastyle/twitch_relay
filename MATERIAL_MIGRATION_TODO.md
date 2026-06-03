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

### Pending: Phase 3 - Shared MUI Primitives

Convert reusable UI primitives before broad page work.

Files likely in scope:

- `web/src/components/ui/confirm-dialog.tsx`
- `web/src/components/ui/error-state.tsx`
- `web/src/components/ui/empty-state.tsx`
- `web/src/components/ui/media-row.tsx`
- `web/src/components/ui/loaded-fade.tsx`
- `web/src/components/ui/skeleton-media-list.tsx`
- `web/src/components/ui/skeleton-recording-list.tsx`
- `web/src/components/ui/skeleton-text.tsx`
- `web/src/components/ui/skeleton-thumbnail.tsx`
- `web/src/components/ui/skeleton-video-list.tsx`
- `web/src/components/ui/index.ts`

Targets:

- Replace custom dialog with MUI `Dialog`.
- Replace error/empty states with MUI `Alert`, `Paper`, `Stack`, `Typography`.
- Replace skeletons with MUI `Skeleton` where it improves consistency.
- Decide whether shared media rows become dense `CardActionArea` or `ListItemButton` wrappers.

Acceptance criteria:

- Existing pages still render all loading, empty, error, and confirm states.
- Dialog focus behavior and escape/confirm/cancel behavior are correct.
- No old component CSS remains for primitives that were fully migrated.
- Verification commands pass.

### Pending: Phase 4 - App Shell, Header, And Navigation

Files likely in scope:

- `web/src/components/shared/app-header.tsx`
- `web/src/components/shared/relay-header.tsx`
- `web/src/components/shared/app-header-actions.tsx`
- `web/src/components/shared/nav-tabs.tsx`
- `web/src/components/layout/twitch-layout.tsx`
- `web/src/components/layout/you-tube-layout.tsx`
- `web/src/components/layout/app-version.tsx`
- `web/src/components/youtube/you-tube-shell.tsx`

Targets:

- Convert header chrome to MUI `AppBar`, `Toolbar`, `Button`, `IconButton`, `Menu`, `Typography`.
- Convert YouTube nav to MUI `Tabs`.
- Replace custom collapsed menu/backdrop logic with MUI menu primitives.
- Preserve route switching between Twitch and YouTube.
- Preserve focus management in layout wrappers.

Acceptance criteria:

- Header actions work desktop and mobile/collapsed.
- Keyboard navigation works.
- Route theme switching remains correct.
- No regression in sign out/connect/disconnect flows.

### Pending: Phase 5 - Twitch Channel Management Surface

Files likely in scope:

- `web/src/pages/twitch-home-page.tsx`
- `web/src/components/twitch/twitch-panel.tsx`
- `web/src/components/twitch/twitch-channels-view.tsx`
- `web/src/components/twitch/channel-card.tsx`
- `web/src/components/twitch/channel-card-helpers.ts`
- `web/src/components/twitch/add-channel-form.tsx`
- `web/src/components/twitch/auth-panel.tsx`
- `web/src/pages/twitch-channel-page.tsx`

Targets:

- Convert channel list screen to dense Material layout.
- Rebuild channel cards as dense MUI `Card`/`Paper` rows.
- Use MUI `TextField`, `Switch`, `Button`, `IconButton`, `Tooltip`, and `Chip`.
- Preserve information density and truncation.
- Preserve touch accessibility for destructive actions.

Acceptance criteria:

- Add channel works.
- Remove channel works.
- Watch stream action works.
- Auto-record toggle works.
- Manual record toggle works.
- Live/offline visual states are clear.
- Mobile/narrow layout is usable.

### Pending: Phase 6 - Twitch Recordings Surface

Files likely in scope:

- `web/src/pages/twitch-recordings-page.tsx`
- `web/src/pages/twitch-recording-player-page.tsx`
- `web/src/components/twitch/recordings-overview.tsx`
- `web/src/components/twitch/recording-filters.tsx`
- `web/src/components/twitch/recording-actions.tsx`
- `web/src/components/twitch/recording-badges.tsx`
- `web/src/components/twitch/active-recording-section.tsx`
- `web/src/components/twitch/completed-recordings-section.tsx`
- `web/src/components/twitch/completed-recording-row.tsx`
- `web/src/components/twitch/incomplete-recordings-section.tsx`
- `web/src/components/twitch/incomplete-recording-row.tsx`
- `web/src/components/twitch/incomplete-section-header.tsx`

Targets:

- Convert filters, badges, recording rows, and actions to MUI.
- Prefer dense `Card`/`Paper` rows unless a MUI `Table` is clearly better.
- Convert destructive and merge flows to the shared MUI dialog primitive.

Acceptance criteria:

- Filters work.
- Delete/merge/repair/play actions still work.
- Recording statuses remain scannable.
- Incomplete/completed sections remain readable.

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
