## GENERAL PRINCIPLES

- **No `#[allow(dead_code)]` attributes**: Dead code should be removed rather than suppressed. If there's a valid reason to keep unused code (e.g., for future use or API completeness), add a comment explaining why.
- Always prefer fixing the root cause (removing truly dead code or using the constants/structs) over suppressing warnings

## RESPONSES

- Keep responses concise and to the point - unless the user asks otherwise

## PLANNING MODE

- Always ask clarifying questions
- Never assume design, tech stack or features
- Use deep-dive sub-agents to assist with research
- Use deep-dive sub-agents to review the different aspects of your plan before presenting to the user

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

## UI DESIGN

- Always follow the UI design system when creating or reviewing components or pages.
- Design System: @DESIGN.md

## SKILLS

Skills provide specialized instructions and workflows for specific tasks. Use the `skill` tool to load a skill when a task matches its description:

- `find-skills` - Helps discover and install agent skills
- `frontend-design` - Create distinctive, production-grade frontend interfaces
- `github-actions-docs` - GitHub Actions workflows, syntax, triggers, and troubleshooting
- `svelte-code-writer` - Svelte 5 documentation lookup and code analysis (MUST use for .svelte files)
- `svelte-core-bestpractices` - Writing fast, robust, modern Svelte code
- `systematic-debugging` - Use when encountering any bug, test failure, or unexpected behavior
- `tdd` - Test-driven development with red-green-refactor loop
- `typescript-advanced-types` - Complex TypeScript type logic and utilities
- `webapp-testing` - Playwright-based testing for local web applications
