# SlateDB

A free, open-source, offline-first desktop database application inspired by
Microsoft Access, built with Tauri v2, Rust, SQLite (rusqlite, bundled),
SvelteKit + Svelte 5, Bun, Tailwind CSS and shadcn-svelte.

## Download and updates

SlateDB is free to use, modify and share under the [MIT license](LICENSE).
[Download releases for Windows, macOS and Linux](https://github.com/Dilgash-zawity/SlateDB/releases).
Older releases are unsigned and can trigger operating-system warnings. Signed,
notarized releases require the maintainer's platform signing credentials; the
new release workflow will not publish installers until those are configured.

The updater checks for new releases on launch and every six hours while the app
is open. Use **Settings → Updates** to turn automatic checks off, check manually,
read release notes, and choose when to install. Downloads are verified with a
public key embedded in the app and must match the version they were signed for.
Database contents are never uploaded. Installing this updater-enabled version
once is required before older installations can receive automatic updates.

For release setup and download verification, see [RELEASING.md](docs/RELEASING.md).

## Using it

1. On first launch, choose a folder. Every database is one SQLite file in it,
   and the folder is remembered.
2. Create a database (or open one from the list). Each database has its own
   page with a sidebar of tables.
3. Create a table. It opens in **Datasheet view**: type straight into the
   grid, use the `*` row for new records, and “Click to Add” for new fields.
   A record is saved when you leave its row (or press Shift+Enter); Esc undoes
   the field, then the whole record. Right-click rows and headers for more.
4. Switch to **Design view** to edit fields in a grid (name, data type,
   description) with Field Properties below; save the design when done.
   Relations (lookups) to other tables are set on the Lookup tab.

5. Create an **app** from the home screen to build screens for your data:
   pages of tables, forms, charts, number cards, filters, buttons and text,
   over one or more databases. Start blank or from a template (Contacts,
   Shop), which also creates a database with sample records. In **Edit**
   mode you add widgets, drag them on a 12-column grid and set them up in the
   panel on the right; **Run** mode is for using the app. Variables connect
   widgets: a filter control or a row click sets one, and other widgets
   filter by it (e.g. pick a company, see its people). Apps are saved as
   files in the folder's `Apps` subfolder.

Shortcuts: ⌘/Ctrl+K search and actions, ⌘/Ctrl+S save, Shift+Enter save the
record, Esc undo. “Open as form” edits one record in a side panel. Light,
dark and system themes are available (dark by default).

## Status

- Database foundation: create (never overwriting), open, close; tables and
  views; rename and delete tables.
- Fields with rules (required, no duplicates, record ID, defaults, validation
  rules), relations between tables, formulas and safe type changes. Structural
  changes use SQLite's table-rebuild procedure inside a transaction.
- Records: add, edit, delete (single or bulk), search, filters, sorting and
  pagination, with plain-language validation messages.
- Apps: pages of widgets over several databases at once, with undo/redo,
  autosave, master-detail through variables, charts and totals, and
  templates. Renaming a table or field (or a database) updates the apps that
  use it; a widget whose table or field is gone says so instead of failing.

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
├── state/        Svelte 5 rune state: workspace, open database, toasts and confirmations
└── components/
    ├── ui/       shadcn-svelte components (generated; update with the CLI)
    └── app/      the app's screens built only from those components
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
