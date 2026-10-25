## SlateDB with automatic updates

This free, MIT-licensed release adds **Settings → Updates**, automatic checks
for new versions, release notes, download progress, and signed update installation.
Saved online databases (PostgreSQL, MySQL, MariaDB, libSQL, and hosted providers
such as Neon, Supabase, Turso, PlanetScale and CockroachDB) can now be browsed,
edited, queried and used by apps like local databases, with server-side paging and
search, a "Test connection" check, and an Overview, Data, Structure and Query view
for each one. Structure changes on online databases are still made with the
server's own tools. Your local databases stay in your selected workspace folder.

This release also redraws the home map with provider logos for online databases,
and adds a colorful default theme with a Focus mode for the original neutral look.

This release refreshes the SlateDB app icon and logo, redrawn so they stay sharp at
small sizes in the window, the taskbar and the installers.

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
| Windows 64-bit | [`SlateDB_0.0.17_x64-setup.exe`](https://github.com/Dilgash-zawity/SlateDB/releases/download/v0.0.17/SlateDB_0.0.17_x64-setup.exe) |
| macOS Apple Silicon | [`SlateDB_0.0.17_aarch64.dmg`](https://github.com/Dilgash-zawity/SlateDB/releases/download/v0.0.17/SlateDB_0.0.17_aarch64.dmg) |
| macOS Intel | [`SlateDB_0.0.17_x64.dmg`](https://github.com/Dilgash-zawity/SlateDB/releases/download/v0.0.17/SlateDB_0.0.17_x64.dmg) |
| Ubuntu / Debian 64-bit | [`SlateDB_0.0.17_amd64.deb`](https://github.com/Dilgash-zawity/SlateDB/releases/download/v0.0.17/SlateDB_0.0.17_amd64.deb) |
| Linux 64-bit | [`SlateDB_0.0.17_amd64.AppImage`](https://github.com/Dilgash-zawity/SlateDB/releases/download/v0.0.17/SlateDB_0.0.17_amd64.AppImage) |

If you are using 0.0.8 or an earlier version, install this version manually once.
It can then check for future updates from **Settings → Updates**. Installation
is your choice; the app does not silently restart while you work.

Checksums are in [`SHA256SUMS`](https://github.com/Dilgash-zawity/SlateDB/releases/download/v0.0.17/SHA256SUMS). Update payloads are signed with SlateDB's update key.
