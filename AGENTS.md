# Repository Guidelines

## Project Structure & Module Organization

Database Builder is an offline desktop app using Tauri 2, Rust, bundled SQLite, SvelteKit 3, Svelte 5, and Bun.

- `src/routes/`: workspace and database pages.
- `src/lib/`: typed IPC in `api/`, orchestration in `services/`, framework-free business logic in `logic/`, and rune-based state in `state/`.
- `src/lib/components/app/`: application components; `components/ui/`: generated shadcn-svelte primitives.
- `src-tauri/src/`: thin IPC `commands/`, database operations in `database/`, shared `models/`, and `validation/`.
- `src-tauri/tests/phase1.rs`: SQLite integration tests; frontend tests sit beside their modules.
- `static/`: public assets; `src-tauri/icons/`: desktop icons.

## Build, Test, and Development Commands

- `bun install`: install frontend dependencies.
- `bun run tauri dev`: launch the desktop app with Vite; requires Rust and Tauri platform prerequisites.
- `bun run dev`: start the frontend development server.
- `bun run check`: run Svelte and TypeScript checks; keep errors and warnings at zero.
- `bun run test`: run Bun and Rust tests; `bun run test:ts` and `bun run test:rust` run each separately.
- `bun run build`: build the static frontend.
- `bun run tauri build`: build and package the desktop app.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets`: lint Rust.

## Coding Style & Naming Conventions

Match surrounding formatting: tabs in TypeScript/Svelte and four spaces in Rust. Use strict TypeScript, Svelte 5 runes, PascalCase application components, kebab-case TypeScript modules, and snake_case Rust functions/modules. Import shared frontend modules through `#lib/...`; relative imports also support Bun tests. Update generated UI primitives through the shadcn-svelte CLI. No standalone frontend formatter or linter is configured.

## Testing Guidelines

Use `bun:test` in colocated `*.test.ts` files and Rust `#[test]` functions. Integration tests use temporary SQLite files. Add regression coverage for changed business logic, validation, migrations, and IPC shapes. Run one frontend file with `bun test src/lib/logic/table-draft.test.ts`. No numeric coverage threshold is configured.

## Commit & Pull Request Guidelines

History favors short imperative subjects, such as “Add database rename, duplication, and deletion actions.” Follow that style. PRs should explain behavior changes, link relevant issues, report validation commands/results, and include screenshots for UI changes.

## Data Safety & Architecture

Keep database work in Rust and IPC wrappers thin. Update TypeScript wire types alongside Rust models. Preserve parameterized SQL, identifier validation, backend confirmation flags, and transactional schema changes. Never overwrite existing database files or bypass workspace folder boundaries.
