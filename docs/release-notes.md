## Download SlateDB

SlateDB is free and open source under the MIT license. Download an installer
from the assets below:

| Platform | File |
| --- | --- |
| Windows 64-bit | `SlateDB_*_x64-setup.exe` |
| macOS Apple Silicon | `SlateDB_*_aarch64.dmg` |
| macOS Intel | `SlateDB_*_x64.dmg` |
| Ubuntu / Debian 64-bit | `SlateDB_*_amd64.deb` |
| Linux 64-bit | `SlateDB_*_amd64.AppImage` |

New releases use Apple Developer ID signing and notarization on macOS and
Authenticode signing on Windows. Every update payload is signed with SlateDB's
update key, and the app verifies both its signature and its version before
installation. Linux updates select the matching Debian or AppImage package.

Use **Settings → Updates** to check for updates, see release notes, and install
when ready. Existing versions without the updater need one manual installation
of this version first. Your workspace files remain in the folder you selected.

Checksums are in `SHA256SUMS`. Release assets have GitHub build attestations:
`gh attestation verify <downloaded-file> --repo Dilgash-zawity/SlateDB`.
Signing identifies the publisher and protects file integrity; Windows may still
show SmartScreen warnings while a new publisher builds reputation.
