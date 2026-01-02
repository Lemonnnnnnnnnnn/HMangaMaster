# Repository Guidelines

## Project Structure & Module Organization
- `src/` contains the Vue 3 + TypeScript frontend. Route pages live in `src/views/`, reusable UI in `src/components/`, styles and images in `src/assets/`, and routing in `src/router/`.
- `src-tauri/` contains the Rust backend and Tauri config. Key entry points include `src-tauri/src/main.rs` and `src-tauri/src/commands.rs`, with core modules under `src-tauri/src/crawler/`, `src-tauri/src/services/`, and `src-tauri/src/config/`.
- `public/` stores static assets; `dist/` is the Vite build output.

## Build, Test, and Development Commands
Use `pnpm` (see `package.json`).
- `pnpm dev`: start the Vite dev server (port 1420, HMR 1421).
- `pnpm build`: type-check and build the frontend.
- `pnpm preview`: preview the production build locally.
- `pnpm tauri dev`: run the Tauri app in development (frontend + Rust backend).
- `pnpm tauri build`: package the native app.
- `eslint src/`, `prettier --write src/`, `vue-tsc --noEmit`: lint, format, and type-check.

## Coding Style & Naming Conventions
- Vue SFCs use `<script setup lang="ts">` with Tailwind utility classes.
- Follow existing formatting patterns (notably 4-space indents in templates and semicolons in TS).
- Component files use PascalCase (e.g., `src/components/TaskList.vue`).
- Store/service modules use camelCase under `src/views/**/stores` and `src/views/**/services`.
- Rust modules use snake_case (e.g., `src-tauri/src/task/manager.rs`).

## Testing Guidelines
- No test framework is currently configured.
- If you add tests, colocate them with features (for example, `src/**/__tests__/`) and document new commands here.

## Commit & Pull Request Guidelines
- Commit messages follow Conventional Commits (`feat:`, `fix:`, `refactor:`).
- PRs should include a short summary, steps to verify, and screenshots for UI changes.
- Link related issues or tasks when applicable.

## Architecture Notes
- Frontend-to-backend calls go through Tauri commands in `src-tauri/src/commands.rs`.
- Add new site parsers under `src-tauri/src/crawler/parsers/`.
