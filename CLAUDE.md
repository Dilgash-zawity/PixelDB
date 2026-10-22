# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Offline desktop database app in the spirit of MS Access: Tauri v2 + Rust (rusqlite, bundled SQLite) backend, SvelteKit 3 + Svelte 5 (runes) frontend, Tailwind v4 + shadcn-svelte UI, Bun as package manager.

## Commands

```sh
bun run tauri dev          # run the desktop app (also: bun run ddev)
bun run check              # svelte-check / TypeScript — keep at 0 errors, 0 warnings
bun run test               # all tests: bun test src && cargo test
bun test src/lib/logic/table-draft.test.ts            # one TS test file
bun test src -t "renames propagate"                   # TS tests by name
cargo test --manifest-path src-tauri/Cargo.toml --test phase1 add_modify   # one Rust integration test
cargo test --manifest-path src-tauri/Cargo.toml --lib validation::values   # Rust unit tests by path
cd src-tauri && cargo clippy --all-targets            # keep warning-free
bunx tauri build --debug --no-bundle                  # build the app binary without installers
bunx --bun shadcn-svelte@latest add <component>       # add a UI component
```

The shadcn-svelte CLI prompts interactively and has no TTY in agent shells; drive it with `expect`, answering `y` to "Ready to install components and dependencies?" (Enter alone cancels it) and Enter on idle for other prompts.

## Architecture

**IPC boundary.** All database work is in Rust. `src-tauri/src/commands/*` are thin `#[tauri::command]` wrappers that run `database::*` functions on a blocking thread via `DbManager::run`. The `database/` functions take a plain `&rusqlite::Connection` and know nothing about Tauri, which is why `src-tauri/tests/phase1.rs` can test them against real temp-file databases. Errors are `errors::AppError`, serialized as `{ kind, message }`; the frontend turns them into `ApiError` (`src/lib/api/client.ts`) and toasts (`state/feedback.svelte.ts`, with `logic/messages.ts` rewriting raw SQLite constraint text into plain language).

**Wire types are hand-mirrored.** `src/lib/api/types.ts` mirrors `src-tauri/src/models/*`. Serde shapes (tagged enums like `FieldType`, `DefaultValue`, `RecordKey`) are pinned by `wire_format_tests` in `models/mod.rs`; update both sides together. Tauri converts command argument names to camelCase on the JS side (`newName`, `confirmDataLoss`), while nested struct fields stay snake_case.

**Workspace model.** The user picks a folder once (remembered in the app config dir by `workspace::WorkspaceStore`). Databases are created/opened **by name** inside that folder — the frontend never passes database file paths, and names are resolved from a directory listing, not by joining user input. Only one database is open at a time (`DbManager` holds `Option<OpenDatabase>`).

**Access-style types on SQLite.** SQLite has no Currency/Yes-No/etc., so each table's `TableDef` (field types, descriptions, relations) is stored as JSON in the hidden `__app_tables` table alongside the exact `CREATE TABLE` text it produced. `schema::load_table` trusts that metadata only while the stored SQL matches `sqlite_schema`; otherwise it infers a definition from pragmas and reports `unsupported_features` (CHECKs, collations, WITHOUT ROWID…), and structural redesigns of such tables are refused so constraints are never silently dropped. Names starting `sqlite_` or `__app_` are reserved and hidden.

**Schema changes** (`database/migrations.rs`). Column renames use `ALTER TABLE RENAME COLUMN` and propagate into stored metadata (including other tables' FKs and lookup display columns). Anything else goes through `save_table_design`, which follows SQLite's 12-step rebuild: foreign keys off (`ForeignKeysOff` guard, set *outside* the transaction), create temp table, copy with type conversions (`convert_expr`, e.g. currency minor-unit scaling), drop, rename under `legacy_alter_table`, recreate indexes/triggers, restore the AUTOINCREMENT sequence, then fail the whole transaction if new `foreign_key_check` violations appear or a previously working view breaks.

**Safety rules baked into the API.** User values are always bound parameters; identifiers go through `validate_identifier` + `quote_ident`; SQL fragments accepted in DDL (CHECK, DEFAULT, formulas) pass `validation::expressions::validate_expression` (no `;`, comments, subqueries, or unbalanced parens). Destructive operations (drop table/field, delete records, design saves that remove fields) require an explicit `confirm` flag in the backend, not just a UI prompt; new database files use `create_new` so nothing is overwritten. Currency crosses the API as integer minor units or decimal strings parsed exactly.

**Frontend layers** (`src/lib/`):
- `services/` — typed wrappers around `invoke`.
- `logic/` — framework-free, unit-tested business logic: `table-draft.ts` (design edits → `TableDesign` with rename tracking, used by `relationships.ts` for single-field edits), `record-buffer.ts` (per-record edit buffer with field/record undo and submission rules), `filters.ts`, `lookups.ts`, `values.ts`. Put new non-UI logic here with a `*.test.ts`. Use `jsonClone`, not `structuredClone` — the latter throws on Svelte `$state` proxies.
- `state/` — rune-based singletons: `workspace`, `session` (open DB, current table, `tab: 'diagram' | 'datasheet'`), `feedback` (toasts + a promise-based `confirm()`).
- Theme tokens live in `src/app.css` (blue primary, plus custom `--key` for primary-key icons and `--canvas`/`--canvas-dot` for the diagram). The logo is `src/lib/assets/logo.svg`; app icons in `src-tauri/icons` are generated from a 1024px render of it with `bunx tauri icon`.
- `components/ui/` — generated shadcn-svelte components; change via the CLI rather than hand edits. `components/app/` — app screens composed only from those.

**Screens.** `routes/+page.svelte` is the folder/database list (the app is named **Base**); `routes/db/+page.svelte?name=…` is a database (query param keeps the route prerenderable). There is no sidebar and no Design view: the database page has a top bar (database switcher with "New database…", search, theme) and two tabs, `session.tab`: **Diagram** (`RelationshipsView.svelte`) and **Datasheet** (`RecordsView.svelte`, with a table picker in the top bar). All structure editing happens on the Diagram: add/rename/delete tables (dialogs), click a field or "Add field" to open `FieldDrawer.svelte` (name, type and rules via `FieldProperties.svelte`; saves immediately through `fieldDesign`/`withoutField`), click a table or line to open the details drawer (`RelationshipPanel.svelte` inside a right-hand Drawer), drag a field's dot onto another table to propose a link (`logic/relationships.ts` `proposeLink`). Every link change is a `save_table_design` of the child table (FK + optional lookup field, or a junction table for many-to-many). Card positions are saved in `__app_meta` (`metadata::save_layout`). The Datasheet's column menu also opens `FieldDrawer`. Prefer drawers (and dialogs for short prompts); a confirm opened inside a drawer must pass that drawer's `scope` and the drawer renders `<ConfirmHost scope=…>`. The Datasheet registers `session.leaveGuard` so switching tables/tabs/databases first commits the row; navigation is serialized so a guard can't run twice. The command palette talks to the open datasheet through `session.request(...)` / `session.intent`.

## Conventions and gotchas

- SvelteKit 3 removed `$lib`; use the `#lib/...` package import (shadcn components import `#lib/utils.js`). Within `src/lib`, relative imports are also used so `bun test` resolves them.
- `adapter-static` with `fallback: 'index.html'`, `ssr = false`, `prerender = true` in `+layout.ts`.
- Follow the project shadcn-svelte skill (`.claude/skills/shadcn-svelte`): `class` only for layout, semantic color tokens, `gap-*` not `space-*`, `data-icon` on icons inside buttons, items inside their `*.Group`, no manual z-index on overlays. A confirm opened from inside a Sheet or Drawer must use a scoped `<ConfirmHost scope=…>` rendered inside its content, or it stacks behind it.
- Don't `bind:checked` to a possibly-undefined property (Svelte throws `props_invalid_value`); use `checked={!!x}` + `onCheckedChange`.
- Dark mode is the first-run default via a tiny script in `src/app.html` that seeds mode-watcher's localStorage key; `defaultMode` alone doesn't work in this client-only app.
- Tauri capabilities expose only `core:default` and `dialog:allow-open` (folder picker); there is no filesystem plugin. Production CSP is set in `tauri.conf.json` (`devCsp` is null for Vite HMR).
