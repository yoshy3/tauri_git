# AGENTS.md

This repository is a minimal desktop Git client built with Tauri, Svelte, and Rust.

## Goal

- Keep the app compact and dependable.
- Prefer small, targeted changes over broad refactors.
- Preserve the existing UX style unless a task explicitly asks for redesign.

## Stack

- Frontend: Svelte 4 + TypeScript + Vite
- Desktop shell/backend: Tauri 2 + Rust
- Localization: `svelte-i18n`
- Git integration: `git2` plus selected system `git` command execution

## Repository Layout

- `src/`: Svelte app entry, components, styles, diff helpers, and i18n resources
- `src/lib/components/`: main UI panes and dialogs
- `src/lib/i18n/locales/`: English and Japanese translation files
- `src-tauri/src/commands.rs`: Tauri command surface exposed to the frontend
- `src-tauri/src/git/`: Git operations grouped by concern
- `src-tauri/src/models.rs`: shared backend data structures

## Common Commands

```bash
npm install
npm run build
npm run tauri dev
cargo check --manifest-path src-tauri/Cargo.toml
```

## Working Agreement

- Read the relevant Svelte and Rust entry points before editing behavior across the frontend/backend boundary.
- Keep changes localized to the feature area when possible.
- Do not remove existing Japanese or English strings without updating both locale files.
- When adding new UI text, update both `src/lib/i18n/locales/en.json` and `src/lib/i18n/locales/ja.json`.
- Prefer matching existing naming, file placement, and component structure.
- Preserve the current pane-based desktop layout and compact workflow unless the task says otherwise.

## Source Code Style

- Follow the style already present in the touched file instead of reformatting neighboring code unnecessarily.
- Keep functions focused and prefer small helper functions when a block starts mixing UI state, Git operations, and formatting logic.
- Use descriptive names that reflect Git concepts already used in the app, such as repository, branch, remote, tag, stash, and status.
- Prefer explicit behavior over clever abstractions, especially around destructive Git actions and confirmation flows.
- Keep comments sparse and use them only when intent is not obvious from the code.
- Treat roughly 1000 lines as a signal to review file size; if a source file grows beyond that, consider splitting it by responsibility.

### Svelte and TypeScript

- Match the existing style of double quotes and semicolons.
- Prefer straightforward local state and small utility functions over introducing extra abstraction layers.
- Keep component responsibilities clear: UI rendering in components, reusable transformation logic in `src/lib/`.
- When adding user-facing text, route it through i18n keys instead of hardcoding strings in components.
- Prefer nullable values and simple guards that match the current codebase style over complex generic utilities.

### Rust

- Keep Tauri command handlers thin and place Git behavior in `src-tauri/src/git/` modules.
- Prefer structured return types from `models.rs` over loosely formatted strings.
- Propagate errors with clear, user-facing messages that match the existing bilingual error approach.
- Favor straightforward ownership and borrowing patterns over premature optimization.
- Keep module boundaries aligned with Git concerns such as status, history, branches, refs, remotes, and diff.

## Frontend Notes

- Main UI lives in `src/App.svelte` and `src/lib/components/*.svelte`.
- Shared styling is in `src/app.css`; prefer extending existing tokens and patterns instead of introducing one-off styling systems.
- Diff rendering logic lives in `src/lib/diff/sideBySideDiff.ts`.

## Backend Notes

- Add or change Tauri commands in `src-tauri/src/commands.rs`, then wire supporting logic into `src-tauri/src/git/` modules as needed.
- Keep Rust modules focused by Git concern such as history, status, branches, remotes, refs, and diff.
- Prefer returning structured data through existing models instead of ad hoc string parsing in the frontend.

## Validation

- For frontend-only changes, run `npm run build`.
- For Rust/backend changes, run `cargo check --manifest-path src-tauri/Cargo.toml`.
- For changes that cross the Tauri boundary, run both when feasible.
- If a change affects localized UI text, verify both English and Japanese keys exist.

## Documentation

- Do not rewrite existing documentation text in `README.md`, `README_ja.md`, or `TODO.md` without a specific reason tied to the task.
- Unless explicitly requested, avoid stylistic rewrites, paraphrasing, or tone-only edits in existing documentation.
- When documentation updates are required, keep edits narrowly scoped to the affected feature, behavior, or instruction.

## Scope Guardrails

- Do not introduce heavy dependencies without a clear need.
- Avoid unrelated formatting churn.
- Keep behavior explicit for destructive Git actions and preserve confirmation flows.
