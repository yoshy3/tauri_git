# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

A minimal desktop Git client built with Tauri 2 (Rust backend) + Svelte 4 + TypeScript frontend.

## Commands

```bash
npm install                                          # install frontend deps
npm run tauri dev                                    # dev: watch frontend + open Tauri window
npm run build                                        # TypeScript + Vite build only (frontend validation)
npm run tauri build                                  # full production bundle (frontend + Rust release)
cargo check --manifest-path src-tauri/Cargo.toml    # Rust type-check without building
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings  # Rust linting
```

There is no automated test suite. Validation is `npm run build` for frontend-only changes, `cargo check` for Rust changes, and both when crossing the Tauri boundary.

## Architecture

The frontend communicates with the Rust backend exclusively through Tauri's async command invocations (`invoke('command_name', args)`). All Git operations happen in Rust; the frontend is purely UI and state.

**Frontend (`src/`)**
- `App.svelte` — root component; owns all reactive state (repo, UI layout, loading flags, caches, preferences)
- `src/lib/components/` — the main panes: `TopBar`, `SidebarPane`, `HistoryPane`, `CommitPane`, `DiffCompareDialog`, plus branch/tag/stash dialogs
- `src/lib/diff/sideBySideDiff.ts` — unified diff → side-by-side rendering
- `src/lib/i18n/locales/{en,ja}.json` — all user-facing strings; both files must be kept in sync

State management is plain Svelte reactivity in `App.svelte`—no external store library. Preferences (theme, locale, pane widths) persist to `localStorage`. The UI auto-polls every 2.5 s when a dirty flag is set.

**Backend (`src-tauri/src/`)**
- `commands.rs` — all Tauri command handlers; keep these thin
- `git/` — Git logic split by concern: `repository`, `status`, `history`, `branches`, `refs`, `remotes`, `diff`, `worktree`
- `models.rs` — serializable structs returned to the frontend (`GitStatusResponse`, `GitCommitDetail`, etc.)
- `app.rs` — window setup, macOS menu, window-state plugin wiring

The backend uses `git2` (libgit2) for most operations. A few operations fall back to system `git`:
- Rebase (`git rebase`) — libgit2 doesn't expose the rebase API
- Force-push (`--force-with-lease`) — libgit2 push doesn't support it
- git-crypt detection — checked via shell

Commit history is paginated (100 commits per request) to keep the UI responsive.

Error messages follow a bilingual convention (Japanese + English) using the `bilingual()` helper in `git/mod.rs`. New error messages should match this pattern.

## Key Conventions

- **Localization is mandatory:** all user-visible strings must use `svelte-i18n` keys (`$_('key')`). Never hardcode strings in components. When adding text, update both `en.json` and `ja.json`.
- **Tauri command handlers stay thin:** business logic belongs in `src-tauri/src/git/` modules, not in `commands.rs`.
- **Structured return types over strings:** use or extend `models.rs` structs instead of returning ad hoc strings from commands.
- **Destructive actions require confirmation flows:** preserve existing confirmation dialogs; don't bypass them.
- **File size guardrail:** ~1000 lines signals a file should be split by responsibility.
- **Style:** match the existing file—double quotes and semicolons in TS/Svelte, `cargo fmt`-compliant Rust.
