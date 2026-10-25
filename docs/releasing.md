# Releasing

For the maintainer only.

## Who builds what

| Target | Built by | Signatures |
| --- | --- | --- |
| Windows x64 and ARM64 | **The maintainer's Windows machine** (`npm run release`; ARM64 cross-compiled) | Authenticode with the local certificate + updater |
| Linux x64 / ARM64 | CI (`ubuntu-24.04`, `ubuntu-24.04-arm`): `.deb`, AppImage, `.rpm` | Updater |
| macOS Intel / Apple Silicon | CI (`macos-15-intel`, `macos-15`): `.dmg` | No updater; *ad hoc*, or Developer ID + notarization if the `APPLE_*` secrets are set |

The Windows Authenticode certificate is kept: `setup-signing.ps1` creates it
**non-exportable**, so the Windows installers (x64 and ARM64) are signed on the
maintainer's Windows machine and CI reuses them byte for byte from the draft (the
`windows-11-arm` runner only installs and tests the ARM64 one). That way the updater
key does not leave that machine for Windows either.

To build ARM64 on an x64 machine you need, once, `rustup target add
aarch64-pc-windows-msvc` and the "MSVC ARM64 build tools" component of Visual Studio Build
Tools. `npm run release` downloads the ARM64 yt-dlp and deno (`tools.lock.json`) and
puts them in `tools/` only while it builds that installer.

## Setup (once)

In **Iznardo/xp-music → Settings**:

1. **Environments → New environment → `release`**, with **Required reviewers** (you) and
   **Deployment branches and tags** limited to the `main` branch (the release workflow is
   started from `main`). The secrets go **only** here, not in the repository secrets:
   that way no job receives them without your approval.
2. **Secrets and variables → Actions → Variables** (repository level) for the two variables.

| Where | Type | Name | Value |
| --- | --- | --- | --- |
| Repository | Variable | `XP_MUSIC_UPDATER_PUBKEY` | Contents of the existing public key `~/.xp-music/updater.key.pub` |
| Repository | Variable | `WINDOWS_SIGNING_THUMBPRINT` | Thumbprint from `src-tauri/signing.local.json` |
| Environment `release` | Secret | `TAURI_SIGNING_PRIVATE_KEY` | Contents of the **existing** private key `~/.xp-music/updater.key` (not a path) |
| Environment `release` | Secret | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Its password; empty if it has none |
| Environment `release` | Secret (optional) | `APPLE_CERTIFICATE` | "Developer ID Application" certificate exported as `.p12`, in base64 |
| Environment `release` | Secret (optional) | `APPLE_CERTIFICATE_PASSWORD` | Password of the `.p12` |
| Environment `release` | Secret (optional) | `APPLE_SIGNING_IDENTITY` | Identity name, e.g. `Developer ID Application: Name (TEAMID)` |
| Environment `release` | Secret (optional) | `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` | For notarization: Apple ID, app-specific password and Team ID |

Do not generate another updater key: the installed versions trust the current one.
Secrets are only passed when building releases, never in PR checks, pushes
or dry runs, and only to the Linux (updater key) and macOS (Apple certificates) jobs;
each release stops until you approve the `release` environment in
Actions → Review deployments. Windows needs none of them: it arrives signed from your machine.
Without `APPLE_CERTIFICATE` and `APPLE_SIGNING_IDENTITY`, macOS is signed *ad hoc* (users
have to get past Gatekeeper, see [Install](install.md)); with them it is signed with a
Developer ID, and notarized if `APPLE_ID`, `APPLE_PASSWORD` and `APPLE_TEAM_ID` are also set.

## Start the release

From `main` on the maintainer's Windows machine, with `origin` pointing to the upstream:

```powershell
npm run release                     # bumps the patch version
npm run release -- -Bump minor
npm run release -- -Bump none -NoPublish  # local test, does not publish or start CI
npm run release -- -Notes "What changes in this version"
```

The script builds and checks the Windows x64 and ARM64 installers, pushes the tag and
prepares a **draft release** with the EXEs, their updater signatures and the notes
(`-Notes`). Then it starts the single workflow:

1. Checks the tag, the version, that it is a draft and that the commit is on `main`,
   before using any key. It only publishes from `Iznardo/xp-music`.
2. Builds, checks and tests the startup of the six targets. On Windows it does not
   rebuild: it checks the Authenticode signature of each EXE in the draft against the
   configured thumbprint and tests it as is. Linux is built with the same public key and
   generates its updater signatures; macOS is built without the updater.
3. Cryptographically verifies **all** updater signatures with `minisign`, uploads the
   files to the draft (except the Windows EXEs, which were already there) and computes
   the SHA-256 of each one.
4. Writes in the release notes, **above** the maintainer's notes, a
   *System (OS and architecture) · SHA-256 · Download* table with the installers; the link
   text is the file name, which already shows the format. The updater files
   (`.sig`, `latest.json`) stay in the release but are not listed. The table goes
   between HTML markers, so retrying the workflow replaces it instead of duplicating it.
5. Creates a single `latest.json` (with only the maintainer's notes) with one entry per
   target and format: `windows-x86_64`, `windows-aarch64`, `linux-{x86_64,aarch64}-{deb,appimage,rpm}`
   (macOS has no entry: it is experimental and does not update itself); the Windows ones
   also keep the formatless key (`windows-x86_64`…) used by existing installations.
6. Publishes the draft only when everything has passed.

If it fails, the draft is not published. Fix the problem and retry **the same tag**
from Actions → Desktop → Run workflow, or:

```bash
gh workflow run release.yml --repo Iznardo/xp-music --ref main -f tag=v1.0.3
```

**Dry run:** starting the workflow *without* a tag (Actions → Run workflow with the field
empty, or `gh workflow run release.yml --ref my-branch`) builds and tests the six targets
without keys or publishing; it is useful to test CI changes on any branch.

Do not publish the draft by hand or run another process that writes
`latest.json`: the workflow is the only publisher. It rejects tags not on `main`,
inconsistent versions, already published releases and publishing from forks.
To enable the updater on an older Linux build without the public key,
first install one of the new signed packages.

## Arch Linux

The `PKGBUILD` in `packaging/arch/` is not part of the release. After each release,
bump `pkgver` (with `pkgrel=1`), run `updpkgsums` in that folder and commit the change.
