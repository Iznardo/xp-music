# Publica una versión de XP Music (specs/007, FR-007-07): sube la versión, compila
# el instalador firmado, crea la release en GitHub y sube latest.json para el updater.
#
#   npm run release                 # patch: 0.1.0 → 0.1.1
#   npm run release -- -Bump minor
#   npm run release -- -Bump none -NoPublish   # solo compilar (prueba local)
#
# Requisitos (una vez): scripts\setup-signing.ps1 (certificado) y la clave del
# updater en %USERPROFILE%\.xp-music\updater.key (+ .pub), creada con
#   npm run tauri signer generate -- -w "$env:USERPROFILE\.xp-music\updater.key"
# La contraseña de la clave se pide al compilar (o TAURI_SIGNING_PRIVATE_KEY_PASSWORD).
param(
    [ValidateSet('patch', 'minor', 'major', 'none')][string]$Bump = 'patch',
    [switch]$NoPublish,
    [string]$Notes = ''
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
$repo = 'Iznardo/xp-music'
$keyPath = Join-Path $env:USERPROFILE '.xp-music\updater.key'
$signingPath = Join-Path $root 'src-tauri\signing.local.json'
$confPath = Join-Path $root 'src-tauri\tauri.conf.json'

function Fail($msg) { Write-Host "ERROR: $msg" -ForegroundColor Red; exit 1 }
function Write-Utf8($path, $text) { [IO.File]::WriteAllText($path, $text, (New-Object Text.UTF8Encoding $false)) }

# --- 1. Comprobaciones --------------------------------------------------------
if (-not $NoPublish) {
    if (git status --porcelain) { Fail 'Hay cambios sin confirmar.' }
    if ((git rev-parse --abbrev-ref HEAD) -ne 'main') { Fail 'Las releases se hacen desde main.' }
    gh auth status 2>$null | Out-Null
    if ($LASTEXITCODE -ne 0) { Fail 'gh no tiene sesión iniciada (gh auth login).' }
    if (-not (Test-Path $signingPath)) { Fail 'Falta la firma: ejecuta scripts\setup-signing.ps1.' }
    if (-not (Test-Path $keyPath)) { Fail "Falta la clave del updater en $keyPath." }
}
if (-not (Test-Path (Join-Path $root 'tools\yt-dlp\yt-dlp.exe')) -or -not (Test-Path (Join-Path $root 'tools\deno.exe'))) {
    & (Join-Path $PSScriptRoot 'fetch-tools.ps1')
}

# --- 2. Versión ---------------------------------------------------------------
$pkg = Get-Content package.json -Raw | ConvertFrom-Json
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
    foreach ($f in 'package.json', 'src-tauri\tauri.conf.json') {
        Write-Utf8 (Join-Path $root $f) ((Get-Content $f -Raw).Replace($from, $to))
    }
    $cargo = Get-Content src-tauri\Cargo.toml -Raw
    $cargo = ([regex]'(?m)^version = "[^"]+"').Replace($cargo, "version = `"$v`"", 1)
    Write-Utf8 (Join-Path $root 'src-tauri\Cargo.toml') $cargo
    Write-Host "Versión $($pkg.version) → $v"
}

# --- 3. Config temporal: firma y artefactos del updater -----------------------
$extra = @{ bundle = @{ windows = @{} } }
if (Test-Path $signingPath) {
    $sig = Get-Content $signingPath -Raw | ConvertFrom-Json
    $extra.bundle.windows = @{
        certificateThumbprint = $sig.certificateThumbprint
        digestAlgorithm = 'sha256'
        timestampUrl = 'http://timestamp.digicert.com'
    }
} else {
    Write-Warning 'Sin certificado: el instalador no irá firmado.'
}
if (Test-Path $keyPath) {
    $extra.bundle.createUpdaterArtifacts = $true
    $env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content $keyPath -Raw).Trim()
    $env:XP_MUSIC_UPDATER_PUBKEY = (Get-Content "$keyPath.pub" -Raw).Trim()
    if (-not $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) {
        $sec = Read-Host 'Contraseña de la clave del updater' -AsSecureString
        $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = [Runtime.InteropServices.Marshal]::PtrToStringAuto(
            [Runtime.InteropServices.Marshal]::SecureStringToBSTR($sec))
    }
} else {
    Write-Warning 'Sin clave del updater: esta compilación no podrá autoactualizarse.'
}
$tmpConf = Join-Path ([IO.Path]::GetTempPath()) "xp-music-release-$v.json"
Write-Utf8 $tmpConf ($extra | ConvertTo-Json -Depth 5)

# --- 4. Compilar --------------------------------------------------------------
try {
    npm run tauri build -- --config $tmpConf
    if ($LASTEXITCODE -ne 0) { Fail 'La compilación falló.' }
} finally {
    Remove-Item -Force $tmpConf
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY, Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
}
$nsis = Join-Path $root 'src-tauri\target\release\bundle\nsis'
$installer = Get-ChildItem $nsis -Filter "*_${v}_x64-setup.exe" | Select-Object -First 1
if (-not $installer) { Fail "No se encuentra el instalador de $v en $nsis." }
Write-Host "Instalador: $($installer.FullName) ($([math]::Round($installer.Length / 1MB, 1)) MB)"

# --- 5. Verificar firma -------------------------------------------------------
if (Test-Path $signingPath) {
    foreach ($f in $installer.FullName, (Join-Path $root 'src-tauri\target\release\xp-music.exe')) {
        $a = Get-AuthenticodeSignature $f
        if ($a.Status -ne 'Valid' -or $a.SignerCertificate.Subject -ne 'CN=iznardo') {
            Fail "Firma no válida en $f ($($a.Status))."
        }
        if (-not $a.TimeStamperCertificate) { Write-Warning "Sin sello de tiempo en $f." }
    }
    Write-Host 'Firma Authenticode correcta (CN=iznardo).'
}
if ($NoPublish) { Write-Host 'Compilado sin publicar.'; exit 0 }

# --- 6. Commit, tag y release -------------------------------------------------
$tag = "v$v"
if ($Bump -ne 'none') {
    git add package.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json
    git commit -m "release: $tag"
}
git tag $tag
git push origin main $tag
if ($LASTEXITCODE -ne 0) { Fail 'No se pudo subir el tag.' }

$sigFile = "$($installer.FullName).sig"
if (-not $Notes) { $Notes = "XP Music $v" }
gh release create $tag $installer.FullName $sigFile --repo $repo --title "XP Music $v" --notes $Notes
if ($LASTEXITCODE -ne 0) { Fail 'No se pudo crear la release.' }

# Repo público: el updater descarga el instalador por su enlace normal.
$rel = gh api "repos/$repo/releases/tags/$tag" | ConvertFrom-Json
$asset = $rel.assets | Where-Object { $_.name -like "*_${v}_x64-setup.exe" } | Select-Object -First 1
if (-not $asset) { Fail 'No se encuentra el instalador en la release.' }
$latest = [ordered]@{
    version = $v
    notes = $Notes
    pub_date = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    platforms = [ordered]@{
        'windows-x86_64' = [ordered]@{
            signature = (Get-Content $sigFile -Raw).Trim()
            url = $asset.browser_download_url
        }
    }
}
$latestPath = Join-Path ([IO.Path]::GetTempPath()) 'latest.json'
Write-Utf8 $latestPath ($latest | ConvertTo-Json -Depth 5)
gh release upload $tag $latestPath --repo $repo --clobber
Remove-Item -Force $latestPath
Write-Host "Publicada $tag." -ForegroundColor Green
