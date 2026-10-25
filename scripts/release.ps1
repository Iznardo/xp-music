# Publishes an XP Music version (specs/007, FR-007-07): bumps the version, builds
# the signed Windows x64 and ARM64 installers (the only ones built here: the
# certificate is non-exportable), prepares a draft and launches the multi-platform
# workflow. The workflow builds the rest (Linux x64/ARM64, macOS Intel/Apple Silicon),
# also tests the Windows ones, and only it publishes the release, the download table
# and latest.json once every target passes. -Notes are the maintainer's notes: the
# workflow adds the download table above them.
#
#   npm run release                 # patch: 0.1.0 → 0.1.1
#   npm run release -- -Bump minor
#   npm run release -- -Bump none -NoPublish   # build only (local test)
#
# Requirements (once): scripts\setup-signing.ps1 (certificate), the updater key
# at %USERPROFILE%\.xp-music\updater.key (+ .pub), created with
#   npm run tauri signer generate -- -w "$env:USERPROFILE\.xp-music\updater.key"
# and, for ARM64, `rustup target add aarch64-pc-windows-msvc` plus the
# "MSVC ARM64 build tools" component of Visual Studio Build Tools.
# The key password is requested when building (or TAURI_SIGNING_PRIVATE_KEY_PASSWORD).
param(
    [ValidateSet('patch', 'minor', 'major', 'none')][string]$Bump = 'patch',
    [switch]$NoPublish,
    [string]$Notes = ''
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
$repo = 'Iznardo/xp-music'
$armTarget = 'aarch64-pc-windows-msvc'
$keyPath = Join-Path $env:USERPROFILE '.xp-music\updater.key'
$signingPath = Join-Path $root 'src-tauri\signing.local.json'
$confPath = Join-Path $root 'src-tauri\tauri.conf.json'

function Fail($msg) { Write-Host "ERROR: $msg" -ForegroundColor Red; exit 1 }
# Get-Content without -Encoding reads as ANSI in PowerShell 5.1 and mangles accented characters.
function Write-Utf8($path, $text) { [IO.File]::WriteAllText($path, $text, (New-Object Text.UTF8Encoding $false)) }

# --- 1. Checks --------------------------------------------------------
if (-not $NoPublish) {
    if (git status --porcelain) { Fail 'There are uncommitted changes.' }
    if ((git rev-parse --abbrev-ref HEAD) -ne 'main') { Fail 'Releases are made from main.' }
    $origin = git remote get-url origin
    if ($LASTEXITCODE -ne 0 -or $origin -notmatch '^(git@github\.com:|https://github\.com/)Iznardo/xp-music(\.git)?/?$') {
        Fail 'origin must point to Iznardo/xp-music; releases are not published from a fork.'
    }
    gh auth status 2>$null | Out-Null
    if ($LASTEXITCODE -ne 0) { Fail 'gh is not logged in (gh auth login).' }
    if (-not (Test-Path $signingPath)) { Fail 'Signing is missing: run scripts\setup-signing.ps1.' }
    if (-not (Test-Path $keyPath)) { Fail "The updater key is missing at $keyPath." }
}
if (-not (@(rustup target list --installed) -contains $armTarget)) {
    Fail "The ARM64 target is missing: rustup target add $armTarget (and the Visual Studio 'MSVC ARM64 build tools')."
}
# ring and aws-lc-sys need clang/clang-cl to build for ARM64 (cl.exe is not enough). If they
# are not on PATH, use the ones from Visual Studio's "C++ Clang Compiler for Windows".
if (-not (Get-Command clang-cl -ErrorAction SilentlyContinue)) {
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    $clang = if (Test-Path $vswhere) {
        & $vswhere -latest -products * -find 'VC\Tools\Llvm\x64\bin\clang-cl.exe' | Select-Object -First 1
    }
    if (-not $clang) {
        Fail "Clang is missing for the ARM64 build: add the Visual Studio component 'C++ Clang Compiler for Windows' (Microsoft.VisualStudio.Component.VC.Llvm.Clang)."
    }
    $env:PATH = "$(Split-Path $clang);$env:PATH"
}
if (Test-Path (Join-Path $root 'tools.x64')) { Fail 'tools.x64 exists from a previous run: move it back to tools\ or delete it.' }
if (-not (Test-Path (Join-Path $root 'tools\yt-dlp\yt-dlp.exe')) -or -not (Test-Path (Join-Path $root 'tools\deno.exe'))) {
    & (Join-Path $PSScriptRoot 'fetch-tools.ps1')
}

# --- 2. Version ---------------------------------------------------------------
$pkg = Get-Content package.json -Raw -Encoding UTF8 | ConvertFrom-Json
$version = [version]$pkg.version
$new = switch ($Bump) {
    'major' { [version]::new($version.Major + 1, 0, 0) }
    'minor' { [version]::new($version.Major, $version.Minor + 1, 0) }
    'patch' { [version]::new($version.Major, $version.Minor, $version.Build + 1) }
    'none' { $version }
}
$v = $new.ToString()
if ($Bump -ne 'none') {
    $from = '"version": "' + $pkg.version + '"'
    $to = '"version": "' + $v + '"'
    npm version $v --no-git-tag-version --ignore-scripts
    if ($LASTEXITCODE -ne 0) { Fail 'Could not update package.json/package-lock.json.' }
    Write-Utf8 $confPath ((Get-Content $confPath -Raw -Encoding UTF8).Replace($from, $to))
    $cargo = Get-Content src-tauri\Cargo.toml -Raw -Encoding UTF8
    $cargo = ([regex]'(?m)^version = "[^"]+"').Replace($cargo, "version = `"$v`"", 1)
    Write-Utf8 (Join-Path $root 'src-tauri\Cargo.toml') $cargo
    Write-Host "Version $($pkg.version) → $v"
}

# --- 3. Temporary config: signing and updater artifacts -----------------------
$extra = @{ bundle = @{ windows = @{} } }
if (Test-Path $signingPath) {
    $sig = Get-Content $signingPath -Raw | ConvertFrom-Json
    # Tauri looks for signtool in the SDK registry entry, and on some installations
    # that entry points to a folder without it (it fails when checking deno/yt-dlp
    # before signing them). With TAURI_WINDOWS_SIGNTOOL_PATH it uses the given one.
    $signtool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" -ErrorAction SilentlyContinue |
        Sort-Object FullName | Select-Object -Last 1
    if (-not $signtool) { Fail 'signtool.exe not found (install the Windows SDK).' }
    $env:TAURI_WINDOWS_SIGNTOOL_PATH = $signtool.FullName
    $extra.bundle.windows = @{
        certificateThumbprint = $sig.certificateThumbprint
        digestAlgorithm = 'sha256'
        timestampUrl = 'http://timestamp.digicert.com'
    }
} else {
    Write-Warning 'No certificate: the installer will not be signed.'
}
if (Test-Path $keyPath) {
    $extra.bundle.createUpdaterArtifacts = $true
    $env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content $keyPath -Raw).Trim()
    $env:XP_MUSIC_UPDATER_PUBKEY = (Get-Content "$keyPath.pub" -Raw).Trim()
    # The bundler verifies the artifact signature with the key from the config.
    $extra.plugins = @{ updater = @{ pubkey = $env:XP_MUSIC_UPDATER_PUBKEY } }
    if (-not $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) {
        $sec = Read-Host 'Updater key password' -AsSecureString
        $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = [Runtime.InteropServices.Marshal]::PtrToStringAuto(
            [Runtime.InteropServices.Marshal]::SecureStringToBSTR($sec))
    }
} else {
    Write-Warning 'No updater key: this build will not be able to self-update.'
}
$tmpConf = Join-Path ([IO.Path]::GetTempPath()) "xp-music-release-$v.json"
Write-Utf8 $tmpConf ($extra | ConvertTo-Json -Depth 5)

# --- 4. Build (x64 and ARM64) -------------------------------------------------
$tools = Join-Path $root 'tools'
$toolsX64 = Join-Path $root 'tools.x64'
try {
    npm run tauri build -- --config $tmpConf
    if ($LASTEXITCODE -ne 0) { Fail 'The x64 build failed.' }
    # The installer bundles tools\ as is: while ARM64 builds, the ARM64 yt-dlp and deno
    # go there (fetch-tools only checks their SHA-256, it does not run them).
    Move-Item $tools $toolsX64
    try {
        $env:XP_MUSIC_TOOLS_TARGET = 'windows-arm64'
        node scripts/fetch-tools.mjs
        if ($LASTEXITCODE -ne 0) { Fail 'Could not download the ARM64 yt-dlp and deno.' }
        npm run tauri build -- --config $tmpConf --target $armTarget
        if ($LASTEXITCODE -ne 0) { Fail 'The ARM64 build failed.' }
    } finally {
        Remove-Item Env:XP_MUSIC_TOOLS_TARGET -ErrorAction SilentlyContinue
        if (Test-Path $tools) { Remove-Item -Recurse -Force $tools }
        Move-Item $toolsX64 $tools
    }
} finally {
    Remove-Item -Force $tmpConf
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY, Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
}
$installers = foreach ($build in @(
        @{ dir = 'src-tauri\target\release\bundle\nsis'; arch = 'x64' },
        @{ dir = "src-tauri\target\$armTarget\release\bundle\nsis"; arch = 'arm64' })) {
    $nsis = Join-Path $root $build.dir
    $found = Get-ChildItem $nsis -Filter "*_${v}_$($build.arch)-setup.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $found) { Fail "$($build.arch) installer for $v not found in $nsis." }
    Write-Host "Installer: $($found.FullName) ($([math]::Round($found.Length / 1MB, 1)) MB)"
    $found
}

# --- 5. Verify signature -------------------------------------------------------
if (Test-Path $signingPath) {
    # Only the installers: Tauri signs the executable when bundling it and then leaves
    # the unsigned copy in target\...\release (the one inside the installer is signed).
    foreach ($f in @($installers.FullName)) {
        $a = Get-AuthenticodeSignature $f
        if ($a.Status -ne 'Valid' -or $a.SignerCertificate.Subject -ne 'CN=iznardo') {
            Fail "Invalid signature on $f ($($a.Status))."
        }
        if (-not $a.TimeStamperCertificate) { Write-Warning "No timestamp on $f." }
    }
    Write-Host 'Authenticode signature OK (CN=iznardo).'
}
if ($NoPublish) { Write-Host 'Built without publishing.'; exit 0 }

# --- 6. Prepare the draft; the workflow builds the rest, verifies and publishes --
$assets = foreach ($installer in $installers) {
    $sigFile = "$($installer.FullName).sig"
    if (-not (Test-Path $sigFile)) { Fail "The updater signature of $($installer.Name) is missing." }
    $installer.FullName
    $sigFile
}
$tag = "v$v"
if ($Bump -ne 'none') {
    git add package.json package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json
    if ($LASTEXITCODE -ne 0) { Fail 'Could not stage the version files.' }
    git commit -m "release: $tag"
    if ($LASTEXITCODE -ne 0) { Fail 'Could not commit the version.' }
}
git tag $tag
if ($LASTEXITCODE -ne 0) { Fail 'Could not create the tag (if the draft already exists, retry the workflow).' }
git push origin main $tag
if ($LASTEXITCODE -ne 0) { Fail 'Could not push the tag.' }

# Maintainer's notes; the workflow prepends the download table (SHA-256 and links).
if (-not $Notes) { $Notes = "XP Music $v" }
gh release create $tag @assets --repo $repo --draft --title "XP Music $v" --notes $Notes
if ($LASTEXITCODE -ne 0) { Fail 'Could not prepare the draft.' }
gh workflow run release.yml --repo $repo --ref main -f "tag=$tag"
if ($LASTEXITCODE -ne 0) { Fail 'The draft is still unpublished: run release.yml with its tag from Actions.' }
Write-Host "Draft $tag ready. The workflow will test the Windows installers, build and test Linux and macOS, and publish the release with the download table and latest.json if everything passes. Before building Linux and macOS it waits for you to approve the release environment (Actions → Review deployments)." -ForegroundColor Green
