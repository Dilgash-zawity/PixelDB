# SlateDB

A free, open-source, offline-first desktop database application inspired by
Microsoft Access, built with Tauri v2, Rust, SQLite (rusqlite, bundled),
SvelteKit + Svelte 5, Bun, Tailwind CSS and shadcn-svelte.

## Download and updates

SlateDB is free to use, modify and share under the [MIT license](LICENSE).
[Download releases for Windows, macOS and Linux](https://github.com/Dilgash-zawity/SlateDB/releases).
Community releases include signed update payloads, while Apple notarization and
Windows publisher signing are still pending. Operating-system warnings can still
appear. Publisher-verified releases require the maintainer's platform signing
credentials; community releases clearly state their signing status.

The updater checks for new releases on launch and every six hours while the app
is open. Use **Settings → Updates** to turn automatic checks off, check manually,
read release notes, and choose when to install. Downloads are verified with a
public key embedded in the app and must match the version they were signed for.
Database contents are never uploaded. Installing this updater-enabled version
once is required before older installations can receive automatic updates.

For release setup and download verification, see [RELEASING.md](docs/RELEASING.md).

## Using it

The **[user guide](docs/user-guide.md)** covers everything. In short:

1. On first launch, choose a folder. Databases are SQLite files in its `DB`
   subfolder and apps are files in `Apps`. Try the sample shop to explore.
2. A database has four tabs: **Diagram** (tables, fields and links),
   **Datasheet** (records, spreadsheet-style, with paste from Excel, find and
   replace, saved views and undo), **Query** (a visual designer or SQL) and
   **Reports** (grouped, totalled, printable to PDF).
3. Import CSV and Excel files, and export tables and queries to Excel or CSV.
4. Build **apps** over one or more databases: pages of tables, forms, boards,
   charts, filters and buttons, made by hand or built for each table. Share
   an app with its databases as one `.slatepkg` file.
5. Connect **online** PostgreSQL, MySQL and libSQL databases and use them in
   apps like local ones.

Press Ctrl/⌘+K anywhere to search, and ? for keyboard shortcuts.

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
- Data in and out: CSV/Excel import with a column check before writing,
  Excel/CSV export, undo and redo of saved record changes, bulk paste, fill
  and find & replace, saved views, indexes, reports, and app packages.
- Checked at 100,000 records (`cargo test --release --test performance --
  --ignored --nocapture`): pages, sorting and filtering stay well under a
  second.

## Development

```sh
bun install
bun run tauri dev        # run the desktop app
bun run test             # TypeScript (bun test) + Rust tests
bun run check            # svelte-check / TypeScript
bun run test:ui          # UI tests in Chrome against a fake backend (Playwright)
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
