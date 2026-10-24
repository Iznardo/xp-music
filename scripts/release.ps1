# Publica una versión de XP Music (specs/007, FR-007-07): sube la versión, compila
# los instaladores Windows x64 y ARM64 firmados (los únicos que se hacen aquí: el
# certificado es no exportable), prepara un draft y lanza el workflow multiplataforma.
# El workflow compila el resto (Linux x64/ARM64, macOS Intel/Apple Silicon), prueba
# también los de Windows, y solo él publica la release, la tabla de descargas y
# latest.json cuando pasan todos los targets. -Notes son las notas del mantenedor: el
# workflow les añade encima la tabla de descargas.
#
#   npm run release                 # patch: 0.1.0 → 0.1.1
#   npm run release -- -Bump minor
#   npm run release -- -Bump none -NoPublish   # solo compilar (prueba local)
#
# Requisitos (una vez): scripts\setup-signing.ps1 (certificado), la clave del
# updater en %USERPROFILE%\.xp-music\updater.key (+ .pub), creada con
#   npm run tauri signer generate -- -w "$env:USERPROFILE\.xp-music\updater.key"
# y, para ARM64, `rustup target add aarch64-pc-windows-msvc` más el componente
# «MSVC ARM64 build tools» de Visual Studio Build Tools.
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
$armTarget = 'aarch64-pc-windows-msvc'
$keyPath = Join-Path $env:USERPROFILE '.xp-music\updater.key'
$signingPath = Join-Path $root 'src-tauri\signing.local.json'
$confPath = Join-Path $root 'src-tauri\tauri.conf.json'

function Fail($msg) { Write-Host "ERROR: $msg" -ForegroundColor Red; exit 1 }
# Get-Content sin -Encoding lee como ANSI en PowerShell 5.1 y estropea las tildes.
function Write-Utf8($path, $text) { [IO.File]::WriteAllText($path, $text, (New-Object Text.UTF8Encoding $false)) }

# --- 1. Comprobaciones --------------------------------------------------------
if (-not $NoPublish) {
    if (git status --porcelain) { Fail 'Hay cambios sin confirmar.' }
    if ((git rev-parse --abbrev-ref HEAD) -ne 'main') { Fail 'Las releases se hacen desde main.' }
    $origin = git remote get-url origin
    if ($LASTEXITCODE -ne 0 -or $origin -notmatch '^(git@github\.com:|https://github\.com/)Iznardo/xp-music(\.git)?/?$') {
        Fail 'origin debe apuntar a Iznardo/xp-music; no se publica desde un fork.'
    }
    gh auth status 2>$null | Out-Null
    if ($LASTEXITCODE -ne 0) { Fail 'gh no tiene sesión iniciada (gh auth login).' }
    if (-not (Test-Path $signingPath)) { Fail 'Falta la firma: ejecuta scripts\setup-signing.ps1.' }
    if (-not (Test-Path $keyPath)) { Fail "Falta la clave del updater en $keyPath." }
}
if (-not (@(rustup target list --installed) -contains $armTarget)) {
    Fail "Falta el target de ARM64: rustup target add $armTarget (y «MSVC ARM64 build tools» de Visual Studio)."
}
if (Test-Path (Join-Path $root 'tools.x64')) { Fail 'Existe tools.x64 de una ejecución anterior: devuélvelo a tools\ o bórralo.' }
if (-not (Test-Path (Join-Path $root 'tools\yt-dlp\yt-dlp.exe')) -or -not (Test-Path (Join-Path $root 'tools\deno.exe'))) {
    & (Join-Path $PSScriptRoot 'fetch-tools.ps1')
}

# --- 2. Versión ---------------------------------------------------------------
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
    if ($LASTEXITCODE -ne 0) { Fail 'No se pudo actualizar package.json/package-lock.json.' }
    Write-Utf8 $confPath ((Get-Content $confPath -Raw -Encoding UTF8).Replace($from, $to))
    $cargo = Get-Content src-tauri\Cargo.toml -Raw -Encoding UTF8
    $cargo = ([regex]'(?m)^version = "[^"]+"').Replace($cargo, "version = `"$v`"", 1)
    Write-Utf8 (Join-Path $root 'src-tauri\Cargo.toml') $cargo
    Write-Host "Versión $($pkg.version) → $v"
}

# --- 3. Config temporal: firma y artefactos del updater -----------------------
$extra = @{ bundle = @{ windows = @{} } }
if (Test-Path $signingPath) {
    $sig = Get-Content $signingPath -Raw | ConvertFrom-Json
    # Tauri busca signtool en el registro del SDK, y con algunas instalaciones esa
    # entrada apunta a una carpeta sin él (falla al comprobar deno/yt-dlp antes de
    # firmarlos). Con TAURI_WINDOWS_SIGNTOOL_PATH usa el que se le indica.
    $signtool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" -ErrorAction SilentlyContinue |
        Sort-Object FullName | Select-Object -Last 1
    if (-not $signtool) { Fail 'No se encuentra signtool.exe (instala el Windows SDK).' }
    $env:TAURI_WINDOWS_SIGNTOOL_PATH = $signtool.FullName
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
    # El bundler comprueba la firma de los artefactos con la clave de la config.
    $extra.plugins = @{ updater = @{ pubkey = $env:XP_MUSIC_UPDATER_PUBKEY } }
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

# --- 4. Compilar (x64 y ARM64) ------------------------------------------------
$tools = Join-Path $root 'tools'
$toolsX64 = Join-Path $root 'tools.x64'
try {
    npm run tauri build -- --config $tmpConf
    if ($LASTEXITCODE -ne 0) { Fail 'La compilación x64 falló.' }
    # El instalador empaqueta tools\ tal cual: mientras se compila ARM64 se ponen ahí
    # el yt-dlp y el deno de ARM64 (fetch-tools solo comprueba su SHA-256, no los ejecuta).
    Move-Item $tools $toolsX64
    try {
        $env:XP_MUSIC_TOOLS_TARGET = 'windows-arm64'
        node scripts/fetch-tools.mjs
        if ($LASTEXITCODE -ne 0) { Fail 'No se pudieron descargar yt-dlp y deno de ARM64.' }
        npm run tauri build -- --config $tmpConf --target $armTarget
        if ($LASTEXITCODE -ne 0) { Fail 'La compilación ARM64 falló.' }
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
    if (-not $found) { Fail "No se encuentra el instalador $($build.arch) de $v en $nsis." }
    Write-Host "Instalador: $($found.FullName) ($([math]::Round($found.Length / 1MB, 1)) MB)"
    $found
}

# --- 5. Verificar firma -------------------------------------------------------
if (Test-Path $signingPath) {
    # Solo los instaladores: Tauri firma el ejecutable al empaquetarlo y luego deja en
    # target\...\release la copia sin firmar (la de dentro del instalador sí lo está).
    foreach ($f in @($installers.FullName)) {
        $a = Get-AuthenticodeSignature $f
        if ($a.Status -ne 'Valid' -or $a.SignerCertificate.Subject -ne 'CN=iznardo') {
            Fail "Firma no válida en $f ($($a.Status))."
        }
        if (-not $a.TimeStamperCertificate) { Write-Warning "Sin sello de tiempo en $f." }
    }
    Write-Host 'Firma Authenticode correcta (CN=iznardo).'
}
if ($NoPublish) { Write-Host 'Compilado sin publicar.'; exit 0 }

# --- 6. Preparar draft; el workflow compila el resto, lo verifica y publica -----
$assets = foreach ($installer in $installers) {
    $sigFile = "$($installer.FullName).sig"
    if (-not (Test-Path $sigFile)) { Fail "Falta la firma del updater de $($installer.Name)." }
    $installer.FullName
    $sigFile
}
$tag = "v$v"
if ($Bump -ne 'none') {
    git add package.json package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json
    if ($LASTEXITCODE -ne 0) { Fail 'No se pudieron preparar los archivos de versión.' }
    git commit -m "release: $tag"
    if ($LASTEXITCODE -ne 0) { Fail 'No se pudo confirmar la versión.' }
}
git tag $tag
if ($LASTEXITCODE -ne 0) { Fail 'No se pudo crear el tag (si ya existe el draft, reintenta el workflow).' }
git push origin main $tag
if ($LASTEXITCODE -ne 0) { Fail 'No se pudo subir el tag.' }

# Notas del mantenedor; el workflow antepone la tabla de descargas (SHA-256 y enlaces).
if (-not $Notes) { $Notes = "XP Music $v" }
gh release create $tag @assets --repo $repo --draft --title "XP Music $v" --notes $Notes
if ($LASTEXITCODE -ne 0) { Fail 'No se pudo preparar el draft.' }
gh workflow run release.yml --repo $repo --ref main -f "tag=$tag"
if ($LASTEXITCODE -ne 0) { Fail 'El draft sigue sin publicar: lanza release.yml con su tag desde Actions.' }
Write-Host "Draft $tag preparado. El workflow probará los instaladores de Windows, compilará y probará Linux y macOS, y publicará la release con la tabla de descargas y latest.json si todo pasa. Antes de compilar Linux y macOS esperará a que apruebes el environment release (Actions → Review deployments)." -ForegroundColor Green
