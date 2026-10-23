## SlateDB with automatic updates

This free, MIT-licensed release adds **Settings → Updates**, automatic checks
for new versions, release notes, download progress, and signed update installation.
It also includes the newer app-builder, board, and query improvements added
since version 0.0.8. Your databases stay in your selected workspace folder.

**Signing status:** Update payloads are cryptographically signed with SlateDB's
update key and verified before installation. The Mac apps use ad-hoc signatures
and are **not Apple-notarized**; the Windows installer does **not have a verified
publisher certificate**. macOS may still report that the app is damaged or cannot
be verified, and Windows may show SmartScreen or unknown-publisher warnings.
These warnings are not resolved by the updater signature.

## Downloads

Choose the matching file in the release assets:

| Platform | File |
| --- | --- |
| Windows 64-bit | `SlateDB_0.0.10_x64-setup.exe` |
| macOS Apple Silicon | `SlateDB_0.0.10_aarch64.dmg` |
| macOS Intel | `SlateDB_0.0.10_x64.dmg` |
| Ubuntu / Debian 64-bit | `SlateDB_0.0.10_amd64.deb` |
| Linux 64-bit | `SlateDB_0.0.10_amd64.AppImage` |

If you are using 0.0.8 or an earlier version, install this version manually once.
It can then check for future updates from **Settings → Updates**. Installation
is your choice; the app does not silently restart while you work.

Checksums are in `SHA256SUMS`. Update payloads are signed with SlateDB's update key.
