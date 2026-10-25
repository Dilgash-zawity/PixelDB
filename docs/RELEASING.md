# Signed releases and automatic updates

SlateDB's source and downloads are free under the [MIT license](../LICENSE).
Releases are hosted at <https://github.com/Dilgash-zawity/SlateDB/releases>.
The repository must be public for downloads and update checks without a GitHub
account. Never embed a GitHub token in the application.

## Current prerequisite

Apple and Windows signing credentials have not been provisioned, so releases are
community releases: all four platforms are built and update-signed, but the
installers are not notarized or Authenticode-signed. Never claim publisher
verification for them.
An update signature alone does **not** remove macOS Gatekeeper or Windows
SmartScreen warnings. Existing unsigned downloads are not repaired retroactively.

## Update signing key

The encrypted private key is in `.release-keys/slatedb.key`, its password is in
`.release-keys/slatedb.password`, and the public key is embedded in
`src-tauri/tauri.conf.json`. Private material is ignored by Git and has restricted
file permissions. Back up the private key and password securely outside this
checkout. Losing them breaks the update path for installed copies; never casually
replace the public key in the app.

Set these repository Actions secrets (names only; never put values in source):

| Secret | Value |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | Contents of `.release-keys/slatedb.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Contents of `.release-keys/slatedb.password` |

For local release builds, set `TAURI_SIGNING_PRIVATE_KEY_PATH` and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` securely in the environment. A normal
`bun run tauri build` requires the update signing key. For a local test build
without release signing, use:

```sh
bun run tauri build --config '{"bundle":{"createUpdaterArtifacts":false}}'
```

Development and CI test builds are not public release installers. Automated
checks are disabled in frontend development and browser previews.

## macOS: Developer ID and notarization

Enroll in the Apple Developer Program, obtain a **Developer ID Application**
certificate, export its private key as a password-protected `.p12`, and add:

| Secret | Value |
| --- | --- |
| `APPLE_CERTIFICATE` | Base64-encoded `.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | Export password |
| `APPLE_SIGNING_IDENTITY` | Full `Developer ID Application: … (TEAMID)` identity |
| `APPLE_ID` | Developer account email |
| `APPLE_PASSWORD` | Apple **app-specific** password, not your account password |
| `APPLE_TEAM_ID` | Apple Developer team ID |

Tauri imports the certificate and notarizes/staples the app. CI additionally
checks `codesign`, Gatekeeper assessment and the stapled ticket. Both Apple
Silicon and Intel apps are built. Ad-hoc signing is not a substitute for this.

See [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/).
Enrollment, identity verification, purchases, and accepting terms must be
completed by the account owner.

## Windows: Authenticode

The current release workflow does not sign Windows installers. When credentials exist,
restore the Azure Artifact Signing steps (`artifact-signing-cli`, as documented by Tauri)
from the history of `release.yml`. Provision a public-trust signing account and
certificate profile and authorize an Azure application to sign with that profile.
Service availability depends on the identity and region; if unavailable, use a
trusted code-signing provider with a compatible custom Tauri `signCommand`.

Add Actions secrets `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, and
`AZURE_TENANT_ID`, plus these repository Actions variables:

| Variable | Value |
| --- | --- |
| `AZURE_SIGNING_ENDPOINT` | Your HTTPS `*.codesigning.azure.net` endpoint |
| `AZURE_SIGNING_ACCOUNT` | Signing account name |
| `AZURE_CERTIFICATE_PROFILE` | Public-trust certificate profile name |

The release script validates these variables and generates an ephemeral signing
configuration. Tauri signs the app executable and the NSIS installer before
generating update signatures. CI verifies Authenticode on both outputs.
SmartScreen reputation may still
cause warnings for a newly signed app; signing cannot guarantee none appear.

See [Tauri Windows signing](https://v2.tauri.app/distribute/sign/windows/).

## Publish a version

One workflow, **Build and release** (`.github/workflows/release.yml`), does both:

- Every pull request builds unsigned installers for Windows, Linux and both Mac
  architectures, without secrets. Use this to check a PR builds.
- A manual run from `main` builds the same four platforms with update signatures
  into a draft release, verifies every update signature and version, writes the
  complete `latest.json` and `SHA256SUMS`, records build attestations, and only
  then publishes the release as latest.

To publish:

1. Update `package.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` (the
   SlateDB package), and `src-tauri/tauri.conf.json` to the same new version.
2. Update `docs/community-release-notes.md` with that version's changes, download
   filenames, and accurate signing status. Merge to `main`.
3. Run **Actions → Build and release → Run workflow** from `main`, entering the
   exact version on main.

The run requires the updater signing key and refuses a version that does not
match `main` or a tag that already exists. A failed build leaves the draft
unpublished; rerun the failed jobs for that same release. Do not manually publish
a partial draft or replace artifacts of a published version. Use a new version
for fixes. Older installations check the stable `latest.json` URL over HTTPS;
only newer versions with valid signatures can be installed.

Mac bundles are ad-hoc signed for integrity. Gatekeeper and SmartScreen warnings
may remain. Update signatures remain mandatory on every platform.

## Download verification

Version 0.0.10 was finalized from the GitHub Actions installer builds after the
publishing job could not install Minisign. Its update signatures were verified
before publication, and it includes checksums but no GitHub build attestations.
The workflow installs the pinned official Minisign binary.

On Linux, in the downloaded release directory:

```sh
sha256sum -c SHA256SUMS --ignore-missing
```

For releases with GitHub build attestations:

```sh
gh attestation verify SlateDB_VERSION_amd64.AppImage --repo Dilgash-zawity/SlateDB
```

For independent update-signature verification, download `updater.pub` and the
artifact's `.sig`, decode the `.sig` from base64 to a Minisign signature file,
then run `minisign -Vm <artifact> -p updater.pub -x <decoded-signature>`.
The app performs this verification itself before installing. SHA256 checksums
detect damaged downloads; they are not a substitute for authenticated signatures.
