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

- Global OpenCode skills are available from `~/.agents/skills/`. Currently installed: `rust-best-practices` and `vercel-react-best-practices`. Use them when a task matches before falling back to ad-hoc guidance.

### Agent roles

- `build` is the main coordinator and final decision-maker. It should primarily orchestrate work rather than implement code directly.
- `build` must not write feature or bug-fix code directly when a suitable coding subagent can handle the implementation.
- `plan` is the planning/review coordinator and must not edit code.
- Subagents gather context, review, draft docs, and should handle most code implementation.
- For risky edits, prefer heavy coding agents before falling back to direct `build` edits.
- Subagents may ask before edits when their role requires it.
- Subagents must not bump versions.
- If `build` edits code directly, it must explicitly state why delegation was not used.

### Subagents

- Never implement features yourself when possible - use sub-agents!
- The primary model should mainly orchestrate implementation; delegate coding work to sub-agents whenever feasible.
- Before any non-trivial code edit, delegate implementation or patch design to a coding subagent first.
- Identify changes from the plan that can be implemented in parallel, and use sub-agents to implement the features efficiently
- When using sub-agents to implement features, act as a coordinator only
- Use the best model for the task - premium models for complex tasks (like coding) and mid-tier models for simpler tasks, like documentation
- After completing features (large or small), always run commands like lint, type check and next build to check code quality
- The primary model may edit code directly only for version bumps, lockfile updates, conflict resolution after subagent work, very small mechanical fixes discovered during verification, or when delegation failed or is unavailable.

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
Oxlint uses `web/.oxlintrc.json` with browser and ES2024 env settings; do not assume any separate manual globals list is maintained there.

### Frontend Notes
- The frontend stack is React 19 + Vite+ + TypeScript. See `@DESIGN.md` for the current design system and stack details.
- Use React components in `web/src/components/`.
- Styling uses plain CSS with CSS custom properties. Do not introduce utility-framework or CSS-in-JS assumptions.

## VERSION BUMPING

- When making bug fixes or small improvements that could reasonably constitute a patch release, bump the project version in **both** `Cargo.toml` and `web/package.json` together, and run `cargo update -p twitch-relay` to keep `Cargo.lock` in sync.
- Only bump the minor/major version when the changeset is large enough or includes breaking changes. When in doubt, patch bump.
- Do not leave `Cargo.toml` and `web/package.json` out of sync.
- **Sub-agents are not allowed to bump versions.** Version bumps in `Cargo.toml`, `web/package.json`, and `Cargo.lock` must be done by the main coordinating agent only.

- Always follow the UI design system when creating or reviewing components or pages.
- Design System: @DESIGN.md
