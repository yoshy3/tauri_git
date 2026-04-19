# Tauri Git

[日本語版はこちら](./README_ja.md)

Minimal Git GUI built with Tauri, Svelte, and Rust.

## Overview

Tauri Git is a desktop Git client focused on a compact workflow:

- Open a local Git repository
- Inspect working tree status
- Browse commit history
- Commit changes
- Amend the latest commit from the commit dialog
- Create and apply stashes
- Fetch, pull, and push
- Force-push amended or diverged branches with an explicit confirmation flow
- Show pending pull / push commit counts in the top bar and next to local branches in the sidebar
- Browse branches and remotes in a tree view
- Browse tags and jump from a tag to its commit in history
- Checkout, create, rebase, reset, and delete branches

## Screenshot

![screenshot](images/screenshot-1.png)

## Tech Stack

- Frontend: Svelte + Vite
- Backend: Tauri 2 + Rust
- Git access: `git2` and `git` command execution
- i18n: `svelte-i18n`

## Current Features

- Repository picker and last-opened repository restore
- Recent repository history in the top-bar `Open` split button
- Auto-refresh when repository files or refs change
- Working tree status list
- Commit history with labels for local, remote, and tag refs
- Commit history search and filtering
- Branch search and filtering in the sidebar
- Tag selection in the sidebar with commit selection and scroll-to in history
- Tag creation dialog in WebView
- Tag deletion flow from the sidebar menu
- Optional tag push to `origin` after creation
- Commit detail panel with author / committer / refs / parents / changed files
- Side-by-side diff dialog from commit details
- Side-by-side diff dialog from the changes panel
- `Reset` action from commit details and the top bar, with `soft` / `mixed` / `hard` selection
- Commit creation from the changes panel
- Commit amend from the changes panel, with previous summary / description prefilled
- Collapsed changes panel button with changed-file count badge
- Warning when amending a commit that has already been pushed
- Stash create / apply / pop
- Stash selection and apply / pop from the sidebar
- `Fetch`, `Pull`, `Push`, and `Refresh`
- Optional `--force-with-lease` push flow for amended or diverged branch history
- Top-bar badges for incoming / outgoing commit counts on `Pull` and `Push`
- Incoming / outgoing commit counts next to local branch names in the sidebar
- Local and remote branch tree display with `/`-based nesting
- Branch checkout from local and remote refs
- Branch creation dialog with optional auto-switch
- Rebase current branch onto a selected local or remote branch
- Branch delete dialog implemented in WebView
- Reset current branch to a selected commit with `soft` / `mixed` / `hard`
- Commit history ordering tuned to keep the graph more compact
- Remote branch deletion support
- Safe delete confirmation with branch-name input
- Optional force delete for unmerged local branches
- English / Japanese UI switching
- Dark / light theme switching with saved preference
- Draggable left / center / right pane widths
- Pane width persistence across restarts
- Window size persistence across restarts
- Window position persistence across restarts except on Wayland
- Flicker-free window restore on startup (restores while hidden, then shows)

## Release Build Notes

### macOS

Unsigned or non-notarized macOS builds may be blocked by Gatekeeper with a message such as "The app is damaged and can't be opened."

If you trust the downloaded release, move `Tauri Git.app` into `Applications` and remove the quarantine attribute:

```bash
xattr -dr com.apple.quarantine "/Applications/Tauri Git.app"
```

After that, launch the app again.

Only do this for releases you trust. If the app still does not open, the downloaded file itself may be corrupted.

### Windows

On Windows, Microsoft Defender SmartScreen may show a warning such as "Windows protected your PC" when launching a downloaded release.

If you trust the downloaded release, use this flow:

1. Click `More info`.
2. Confirm that the publisher and file name match the release you downloaded.
3. Click `Run anyway`.

Only do this for releases you trust. If the warning appears for a file from an unexpected source, cancel the launch and verify the download first.

## Development

### Prerequisites

- Node.js
- npm
- Rust toolchain
- Tauri prerequisites for your platform
- Git installed and available in `PATH`

### Install dependencies

```bash
npm install
```

### Run in development

```bash
npm run tauri dev
```

### Build frontend

```bash
npm run build
```

### Verify a Windows release build during development

If you want to test the Windows release app locally, use:

```bash
npm run tauri build
```

Then launch:

```powershell
.\src-tauri\target\release\tauri_git.exe
```

Notes:

- Use `npm run tauri build` for release verification. This bundles the built frontend assets into the Tauri app.
- `cargo build --release` is useful for Rust compile checks, but it is not the right command for verifying the packaged desktop app behavior.
- If you launch an executable produced by `cargo build --release`, the app may try to open `http://localhost:1420` and show `ERR_CONNECTION_REFUSED`.

## Project Structure

```text
src/        Svelte frontend
src-tauri/  Tauri + Rust backend
```

## Notes

- Some Git operations are implemented through the system `git` command for compatibility with familiar workflows.
- The application currently targets a minimal desktop workflow rather than full parity with large Git GUI clients.
- The side-by-side compare view is available from both commit details and the changes panel.
- Standard text diffs are supported, and `git-crypt`-managed files have a dedicated fallback path for compare views.
- Custom diff/filter/textconv setups other than `git-crypt` are not currently supported in compare views and may show no comparable diff.
- Theme preference is persisted and restored automatically on the next launch.
- Pane widths are persisted and restored automatically on the next launch.
- Window size is persisted and restored automatically on the next launch. Window position restore is limited on Linux Wayland sessions.

## License

This project is licensed under the MIT License. See [LICENSE](./LICENSE).
