## SlateDB 0.1.0

This free, MIT-licensed release is SlateDB's first 0.1 version. It adds the
everyday tools for getting data in and out and for working through it.

- **Import** CSV and Excel/OpenDocument files into a table, with a column
  check and row-by-row problems shown before anything is written.
  **Export** tables, filtered views and query results to Excel or CSV.
- **Undo and redo saved changes** in the Datasheet (edits, pastes, imports,
  deletes), and undo card moves on the Diagram.
- **Spreadsheet-style editing**: paste blocks of cells from Excel, copy them
  out, fill down, and find & replace across every matching record.
- **Saved views** keep a table's filter, sort and search under a name.
- **Reports**: a new Reports tab for printable lists grouped by a field, with
  totals per group and overall, printed or saved as PDF.
- **Indexes** can be added to tables to speed up sorting and filtering.
- **Share an app** with its databases as one `.slatepkg` file, and open
  shared apps from the home map.
- **Simpler forms**: field, table, link and record editors are now a single
  page instead of step-by-step wizards.
- **Getting started**: a short tour, a one-click sample shop, and a
  [user guide](https://github.com/Dilgash-zawity/SlateDB/blob/main/docs/user-guide.md).
- **Bug reports**: errors have a *Copy details* button, and Settings → About
  can copy system info and open the log folder.
- Better contrast for warning and success text, visible focus on the
  Datasheet, and the system's reduce-motion setting is respected.

**Signing status:** Update payloads are cryptographically signed with SlateDB's
update key and verified before installation. The Mac apps use ad-hoc signatures
and are **not Apple-notarized**; the Windows installer does **not have a verified
publisher certificate**. macOS may still report that the app is damaged or cannot
be verified, and Windows may show SmartScreen or unknown-publisher warnings.
These warnings are not resolved by the updater signature.

## Downloads

Choose the matching file in the release assets:

| Platform | Download |
| --- | --- |
| Windows 64-bit | [`SlateDB_0.1.0_x64-setup.exe`](https://github.com/Dilgash-zawity/SlateDB/releases/download/v0.1.0/SlateDB_0.1.0_x64-setup.exe) |
| macOS Apple Silicon | [`SlateDB_0.1.0_aarch64.dmg`](https://github.com/Dilgash-zawity/SlateDB/releases/download/v0.1.0/SlateDB_0.1.0_aarch64.dmg) |
| macOS Intel | [`SlateDB_0.1.0_x64.dmg`](https://github.com/Dilgash-zawity/SlateDB/releases/download/v0.1.0/SlateDB_0.1.0_x64.dmg) |
| Ubuntu / Debian 64-bit | [`SlateDB_0.1.0_amd64.deb`](https://github.com/Dilgash-zawity/SlateDB/releases/download/v0.1.0/SlateDB_0.1.0_amd64.deb) |
| Linux 64-bit | [`SlateDB_0.1.0_amd64.AppImage`](https://github.com/Dilgash-zawity/SlateDB/releases/download/v0.1.0/SlateDB_0.1.0_amd64.AppImage) |

If you are using 0.0.8 or an earlier version, install this version manually once.
It can then check for future updates from **Settings → Updates**. Installation
is your choice; the app does not silently restart while you work.

Checksums are in [`SHA256SUMS`](https://github.com/Dilgash-zawity/SlateDB/releases/download/v0.1.0/SHA256SUMS). Update payloads are signed with SlateDB's update key.
