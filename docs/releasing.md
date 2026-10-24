# Releasing

For the maintainer only.

The Windows Authenticode certificate is kept: `setup-signing.ps1` creates it
**non-exportable**, so the installer is still signed on the maintainer's Windows
machine. It is not replaced by an unsigned EXE built in CI.

Configure once in **Iznardo/xp-music → Settings → Secrets and variables → Actions**:

| Type | Name | Value |
| --- | --- | --- |
| Variable | `XP_MUSIC_UPDATER_PUBKEY` | Contents of the existing public key `~/.xp-music/updater.key.pub` |
| Variable | `WINDOWS_SIGNING_THUMBPRINT` | Thumbprint from `src-tauri/signing.local.json` |
| Secret | `TAURI_SIGNING_PRIVATE_KEY` | Contents of the **existing** private key `~/.xp-music/updater.key` (not a path) |
| Secret | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Its password; empty if it has none |

Do not generate another updater key: the versions already installed trust the current one.
The secrets are only used when building releases, never in PR checks.

From `main` on the maintainer's Windows machine, with `origin` pointing to upstream:

```powershell
npm run release                     # bumps patch
npm run release -- -Bump minor
npm run release -- -Bump none -NoPublish  # local test, does not publish or start CI
```

The script builds and verifies the Windows installer, pushes the tag and prepares a
**draft release** with its EXE and updater signature. Then it starts the single workflow:

1. Checks the code on Windows and Linux. Verifies the Authenticode signature of the
   original EXE against the configured thumbprint, without rebuilding or changing it.
2. Builds `.deb` and AppImage with the same public key and generates their signatures.
   Tests both packages outside the repository.
3. Cryptographically verifies the three updater signatures, attaches Linux and creates
   a single `latest.json` with Windows and Linux entries separated by format.
4. Publishes the draft only when everything has passed. The `windows-x86_64` entry is
   kept for existing Windows clients.

If it fails, the draft is not published. Fix the problem and retry **the same tag**
from Actions → Desktop → Run workflow, or:

```bash
gh workflow run release.yml --repo Iznardo/xp-music --ref main -f tag=v1.0.3
```

Do not publish the draft manually or run another process that writes
`latest.json`: the workflow is the only publisher. It rejects tags not on `main`,
inconsistent versions, already published releases and publishing from forks.
To enable the updater in an earlier Linux build without a public key,
first install one of the new signed packages.
