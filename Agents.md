\# Project Overview

This repository represent:

A \*\*desktop application\*\* built with \*\*Tauri v2\*\*, \*\*Svelte 5 runes mode\*\*, \*\*TypeScript\*\*, \*\*TailwindCSS v4\*\*, \*\*shadcn-svelte\*\*, and \*\*lucide\*\* for icons.

\*\*Bun\*\* is used as the runtime, package manager, and task runner across all projects.

\# Setup / Build \& Run

Install dependencies:

```bash

bun install

```

Development:

\*\*Desktop App (Tauri v2)\*\*: `bun tauri dev`

Production Build:

\*\*Desktop App (Tauri v2)\*\*: `bun tauri build`

Type-check the project:

```bash

bun tsc --noEmit

```

\# File-scoped Checks

Type-check a single file:

```bash

bun tsc --noEmit <file>

```

Run ESLint on a single file:

```bash

bun eslint --fix <file>

```

Format a file with Prettier:

```bash

bun prettier --write <file>

```

Run a specific test file:

```bash

bun test <path/to/file.test.ts>

```

\# Code Style \& Conventions

\*\*Language\*\*: TypeScript in strict mode.

\*\*Svelte\*\*: use `<script lang="ts">` in components.

\*\*TailwindCSS v4\*\*: prefer utility-first classes over custom CSS.

\*\*shadcn-svelte\*\*: use provided UI primitives/components consistently.

\*\*Icons\*\*: use \*\*lucide\*\* icons.

\*\*Formatting\*\*: run `bun prettier --write .` before commits.

\*\*Imports\*\*:

Prefer absolute imports from `src/` over long relative paths.

\*\*Naming\*\*:

Components: PascalCase (e.g. `MyButton.svelte`).

Utilities, stores: camelCase or kebab-case as appropriate.

\# Project Structure

`src/` – main application code (Svelte + TypeScript).

`src/components/` – reusable Svelte components.

`src/routes/` – view/route files (if applicable).

`src/lib/` – utilities, hooks, and stores.

`src-tauri/` – only present in desktop apps; contains Rust backend and Tauri config.

`tests/` – unit/integration tests.

\# Testing Guidelines

Use \*\*Vitest\*\* (or compatible runner with Bun).

Add unit tests for new logic.

Run tests before committing:

  ```bash

  bun test

  ```

  ```bash

  bun check

  ```

  ```bash

  bun lint

  ```

\# Do’s \& Don’ts

\## Do:

\- Use TailwindCSS utilities instead of custom CSS.

\- Use shadcn-svelte components for UI consistency.

\- Keep components small and modular.

\- Use lucide icons consistently.

\## Don’t:

\- Don’t use `any` unless unavoidable (add `TODO` if used).

\- Don’t hardcode secrets; use environment variables or config files.

\- Don’t add dependencies without reviewing size and compatibility.

\# Security

\- Exclude `.env` and secrets from version control.

\- For desktop apps, keep secrets in the \*\*Tauri (Rust) backend\*\*, not the frontend.

\- Never expose private APIs or keys in client-side code.

\# External References

General project docs: see `README.md` and/or `docs`.

Tauri v2 docs: \[https://tauri.app](https://tauri.app)

Svelte 5 docs: \[https://svelte.dev](https://svelte.dev)

TailwindCSS v4 docs: \[https://tailwindcss.com/docs/v4](https://tailwindcss.com/docs/v4)

shadcn-svelte docs: \[https://shadcn-svelte.com](https://shadcn-svelte.com)

lucide icons: \[https://lucide.dev](https://lucide.dev)

\# Cleaner Unified List (Svelte)  Virtualization + Scroll

- Symptom: In the Cleaner unified table, users could only scroll 13 rows before content appeared to stick while the scrollbar thumb moved.
- Root causes:
  - Table-based virtualization using spacer `<tr>` rows is fragile across environments; table layout can collapse heights and fight scrolling.
  - Top padding was computed in row-sized steps (`start * rowHeight`), causing bounce at row boundaries.
  - Some containers used `onscroll` instead of Sveltes `on:scroll` directive.

- Fixes implemented (file: `src/routes/cleaner/+page.svelte`):
  - De-duplicate `allItems` by `path` to avoid Svelte `each_key_duplicate` in keyed `{#each}`.
  - Use `on:scroll={...}` for all scroll handlers.
  - Pixel-based top/bottom padding:
    - Introduced `UNIFIED_PREBUFFER = 3` and `unifiedPadPx` tied to `scrollTop`.
    - Top pad follows actual pixels, not just `start * rowHeight`.
    - Bottom pad derives from total height minus top pad minus rendered window height.
  - Virtualization fallback: disabled by default for stability.
    - `let unifiedAutoFallback = true;`
    - `unifiedVirtualize = (!unifiedAutoFallback) && (scanning || allItems.length > 1500)`.

- Re-enable virtualization (optional):
  - Set `unifiedAutoFallback = false` and test with large lists.
  - Tune threshold `(allItems.length > 1500)` and `UNIFIED_PREBUFFER`.
  - Ensure `UNIFIED_ROW_PX` measures stably and container uses `overscroll-behavior: contain`.

- Recommendation (long-term):
  - Prefer a div-based virtualizer over a table:
    - Scroll container (`overflow: auto`) with a fixed-height spacer representing total height.
    - Absolutely position visible rows within the spacer (or offset wrapper).
    - Avoid spacer `<tr>` in `<table>` for virtualization.

- Debug tips:
  - Ensure no effects reset `unifiedStart` during scroll; only update it from user scroll.
  - Streaming flush gates should respect user activity to limit layout thrash.
  - Log `scrollTop`, `unifiedStart`, `unifiedPadPx`, `UNIFIED_ROW_PX` when diagnosing.
  - Keep `overscroll-behavior: contain` on the scroll container.
