# Tauri Git

[日本語版はこちら](./README_ja.md)

Minimal Git GUI built with Tauri, Svelte, and Rust.

## Overview

Tauri Git is a desktop Git client focused on a compact workflow:

- Open a local Git repository
- Inspect working tree status
- Browse commit history
- Commit changes
- Create and apply stashes
- Fetch, pull, and push
- Browse branches and remotes in a tree view
- Checkout, create, and delete branches

## Tech Stack

- Frontend: Svelte + Vite
- Backend: Tauri 2 + Rust
- Git access: `git2` and `git` command execution
- i18n: `svelte-i18n`

## Current Features

- Repository picker and last-opened repository restore
- Working tree status list
- Commit history with labels for local and remote refs
- Commit detail panel with author / committer / refs / parents / changed files
- Side-by-side diff dialog from commit details
- Commit creation from the changes panel
- Stash create / apply / pop
- Stash selection and apply / pop from the sidebar
- `Fetch`, `Pull`, `Push`, and `Refresh`
- Local and remote branch tree display with `/`-based nesting
- Branch checkout from local and remote refs
- Branch creation dialog with optional auto-switch
- Branch delete dialog implemented in WebView
- Remote branch deletion support
- Safe delete confirmation with branch-name input
- Optional force delete for unmerged local branches
- English / Japanese UI switching

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

## Project Structure

```text
src/        Svelte frontend
src-tauri/  Tauri + Rust backend
```

## Notes

- Some Git operations are implemented through the system `git` command for compatibility with familiar workflows.
- The application currently targets a minimal desktop workflow rather than full parity with large Git GUI clients.
- The side-by-side compare view is currently available from commit details and is planned to be reused in the changes panel as well.

## License

This project is licensed under the MIT License. See [LICENSE](./LICENSE).
