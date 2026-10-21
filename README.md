# Database Builder

A free, open-source, offline-first desktop database application inspired by
Microsoft Access, built with Tauri v2, Rust, SQLite (rusqlite, bundled),
SvelteKit + Svelte 5 and Bun. The interface is a dark, Access-style shell
written in plain CSS (no CSS frameworks or component libraries).

## Using it

1. On first launch, choose a folder. Every database is one SQLite file in it,
   and the folder is remembered.
2. Create a blank database or open one from the list; each database opens on
   its own page.
3. Work like in Access: a ribbon (File, Home, Create, Table Design), a
   navigation pane of tables and views, tabbed objects, Datasheet View with
   in-place editing and Design View with Field Properties.

Datasheet keys: Enter moves to the next field, ↑/↓ move between records
(saving the one you leave), Shift+Enter saves, Esc undoes the field and then
the whole record, Delete removes the selected records. ⌘/Ctrl+S saves the
active object, ⌘/Ctrl+W closes it, ⌘/Ctrl+F searches.

## Status

Phase 1 (database foundation) is implemented:

- Create (never overwriting), open, close SQLite database files; read metadata.
- List tables and views; create, rename and delete tables.
- Table designer with Access-style field types, primary/foreign keys, unique,
  NOT NULL, defaults, CHECK rules and field descriptions.
- Add, rename, modify (including type changes) and remove fields, using SQLite's
  table-rebuild procedure inside a transaction where ALTER TABLE cannot.
- Records: insert, read, update, delete, atomic batches, pagination, sorting,
  filtering and search; unsaved-change undo in the editor.

## Development

```sh
bun install
bun run tauri dev        # run the desktop app
bun run test             # TypeScript (bun test) + Rust tests
bun run check            # svelte-check / TypeScript
```

## Field types

| Field type   | SQLite storage                                   |
| ------------ | ------------------------------------------------ |
| Short Text   | `TEXT` + optional `CHECK (length(x) <= n)`       |
| Long Text    | `TEXT`                                           |
| Number       | `INTEGER` or `REAL`                              |
| Large Number | `INTEGER`                                        |
| Currency     | `INTEGER` minor units (e.g. cents), type-checked |
| AutoNumber   | `INTEGER PRIMARY KEY AUTOINCREMENT`              |
| Date/Time    | `TEXT` ISO 8601, checked with `datetime()`       |
| Yes/No       | `INTEGER CHECK (x IN (0, 1))`                    |
| Attachment   | `BLOB`                                           |
| Hyperlink    | `TEXT`                                           |
| Calculated   | generated column (virtual or stored)             |
| Lookup       | `INTEGER`/`TEXT` + foreign key                   |

SQLite cannot store these Access types natively, so each field's type and
description are kept in application metadata (`__app_tables` inside the
database file). Metadata is trusted only while its recorded `CREATE TABLE`
text matches the live schema; otherwise the definition is inferred from
SQLite, and structural changes that could silently drop constraints the
designer cannot represent (e.g. hand-written `CHECK`s) are refused.

Currency values cross the API as integers in minor units, or as decimal
strings in major units (`"12.34"`), which are parsed exactly without floats.

## Layout

```text
src-tauri/src/
├── commands/     Tauri IPC commands (thin, run on blocking threads)
├── database/     connection lifecycle, schema/DDL, migrations, records, metadata, transactions
├── models/       serializable types shared with the frontend
├── validation/   identifiers, paths, SQL expression fragments, values, table definitions
└── errors.rs     structured errors serialized as { kind, message }
src/lib/
├── api/          typed IPC client and TypeScript mirrors of the Rust models
├── services/     database / table / record services, native dialogs
├── logic/        framework-free business logic (table drafts, record buffers, formatting)
├── state/        Svelte 5 rune state: workspace, open database session, dialogs/toasts
└── components/   ribbon, navigation pane, datasheet, design view, dialogs
```

## Safety

- Every user value is bound as a SQL parameter; identifiers are validated and
  quoted; expression fragments for DDL (CHECK, DEFAULT, calculated fields) are
  checked so they cannot escape their parentheses or add statements.
- New database files are created with `create_new`, so existing files are never
  overwritten. Opened files must have a SQLite header.
- Destructive operations (deleting tables, fields or records) require an
  explicit confirmation flag in the backend API, not only in the UI.
- Foreign keys are enforced on every connection; schema rebuilds verify
  `PRAGMA foreign_key_check` and that existing views still compile before
  committing.
- The webview has no filesystem plugin; it can only use the native folder
  picker. Databases are addressed by name inside the chosen folder, never by
  path, so the UI cannot reach files outside it.
