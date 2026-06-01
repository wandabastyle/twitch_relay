## GENERAL PRINCIPLES

- **No `#[allow(...)]` attributes**: Never suppress Clippy or compiler warnings with `#[allow(...)]` attributes. This includes (but is not limited to):
  - `#[allow(dead_code)]` - Dead code should be removed rather than suppressed
  - `#[allow(clippy::too_many_arguments)]` - Refactor to use context structs instead
  - Any other `#[allow(clippy::...)]` - Fix the underlying issue
  
  If there's a valid reason to keep code that triggers warnings (e.g., for future use or API completeness), add a comment explaining why and fix the root cause (removing dead code, using constants/structs, or refactoring) rather than suppressing warnings.

## RESPONSES

- Keep responses concise and to the point - unless the user asks otherwise

## PLANNING MODE

- Always ask clarifying questions
- Never assume design, tech stack or features
- Use deep-dive sub-agents to assist with research
- Use deep-dive sub-agents to review the different aspects of your plan before presenting to the user

## OPENCODE AGENT DELEGATION

Use cheap subagents before spending primary model tokens.

### Default model preference

- Use cheap search/review/docs first.
- Use Kimi for normal build work.
- Use OpenAI mini/fast for cheap focused reasoning.
- Use OpenAI Codex only as fallback or difficult final review.
- Use heavy reasoning only for hard architecture/debugging decisions.

### Subagents

- Use `cheap-grep` for file discovery, symbol lookup, config search, and codebase summaries.
- Use `cheap-review` for first-pass bug checks, obvious lint issues, TODO discovery, and simple refactor review.
- Use `cheap-docs` for README, comments, changelog, and documentation drafts.
- Use `cheap-codex` for small focused patch ideas, local implementation sketches, and narrow code reasoning.
- Use `mid-coder` when cheap agents are not enough, but full primary `build` is still overkill.
- Use `mid-kimi` for focused Kimi-based implementation analysis before final edits.
- Use `heavy-codex` only when Kimi gets stuck or the patch needs OpenAI Codex review.
- Use `heavy-reason` only for hard architecture/debugging decisions, not routine coding.

### Recommended flow

1. `cheap-grep` locates the relevant files and summarizes the current implementation.
2. `cheap-review` checks likely risks or obvious bugs.
3. `cheap-codex` or `mid-coder` gives focused implementation advice if needed.
4. `mid-kimi` analyzes the implementation path when Kimi-style reasoning is useful.
5. `build` applies the final patch.
6. `heavy-codex` or `heavy-reason` is used only if the normal path gets stuck.
7. `plan` or `build` reviews the final result.

Do not use expensive primary models for simple grep, file lookup, config reading, docs drafts, or first-pass review.

The primary model should make final decisions and apply risky edits.
Cheap subagents should gather context and handle low-risk first-pass reasoning.

## CHANGE / EDIT MODE

- Never implement features yourself when possible - use sub-agents!
- Identify changes from the plan that can be implemented in parallel, and use sub-agents to implement the features efficiently
- When using sub-agents to implement features, act as a coordinator only
- Use the best model for the task - premium models for complex tasks (like coding) and mid-tier models for simpler tasks, like documentation
- After completing features (large or small), always run commands like lint, type check and next build to check code quality

## DATABASE SCHEMA CHANGES

- Whenever you make changes to the database schema, ALWAYS run the drizzle generate and migrate commands
- NEVER run drizzle push!

## TESTING

- Use any testing tools, libraries available to the project for testing your changes
- Never assume your changes simply work, always test!
- If the project does not have any testing tools, scripts, MCP tools, skills, etc. available for testing, ask the user whether testing should be skipped.

## CI/CD WORKFLOWS

This repository has GitHub Actions workflows in `.github/workflows/`:
- `app-checks.yml` - Application checks
- `docker-checks.yml` - Docker checks
- `deploy-prod.yml` - Production deployment
- `publish-ghcr.yml` - Publish to GitHub Container Registry

## REMINDERS

- The frontend code is in the `web/` directory. When running web commands like `pnpm run check`, `pnpm run typecheck`, etc., ensure you are in the `web/` directory or use `workdir: "web"` parameter.

## FRONTEND TOOLING (Vite+)

This project uses **Vite+** (vp) for frontend tooling with maximum strictness:

### Running Commands
All frontend commands must be run from the `web/` directory:
```bash
cd web/
pnpm run lint      # Lint with strict rules (--deny-warnings + all categories)
pnpm run fmt       # Format code
pnpm run build     # Production build
pnpm run dev       # Development server
pnpm run test      # Run tests
```

### Strict Lint Configuration
The project uses `--deny-warnings` with all lint categories enabled:
- `-D correctness`
- `-D suspicious` 
- `-D pedantic`
- `-D perf`
- `-D style`
- `-D restriction`
- `-D nursery`

**Never suppress warnings with `eslint-disable` comments** - always fix the underlying issue.

### Global Configuration
Svelte runes (`$props`, `$state`, `$effect`, `$derived`, etc.) and browser globals (`window`, `document`, `fetch`, etc.) are configured in `.oxlintrc.json`.

### Migration Notes
- Migrated from SvelteKit to Vite+ + Svelte-only
- Client-side routing via `src/lib/router/router.svelte.ts`
- No `$app/*` imports - use router exports instead

## VERSION BUMPING

- When making bug fixes or small improvements that could reasonably constitute a patch release, bump the project version in **both** `Cargo.toml` and `web/package.json` together, and run `cargo update -p twitch-relay` to keep `Cargo.lock` in sync.
- Only bump the minor/major version when the changeset is large enough or includes breaking changes. When in doubt, patch bump.
- Do not leave `Cargo.toml` and `web/package.json` out of sync.
- **Sub-agents are not allowed to bump versions.** Version bumps in `Cargo.toml`, `web/package.json`, and `Cargo.lock` must be done by the main coordinating agent only.

- Always follow the UI design system when creating or reviewing components or pages.
- Design System: @DESIGN.md
