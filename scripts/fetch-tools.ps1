# Descarga yt-dlp (versión "onedir") y deno a tools/, en las versiones fijadas en
# tools.lock.json, y comprueba sus SHA-256. Sin argumentos usa el lock; con -Update
# toma las últimas versiones publicadas y reescribe el lock.
#
#   powershell -ExecutionPolicy Bypass -File scripts\fetch-tools.ps1
#   powershell -ExecutionPolicy Bypass -File scripts\fetch-tools.ps1 -Update
param([switch]$Update)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$root = Split-Path -Parent $PSScriptRoot
$tools = Join-Path $root 'tools'
$lockPath = Join-Path $root 'tools.lock.json'
$headers = @{ 'User-Agent' = 'xp-music-build' }

function Get-Sha256($path) { (Get-FileHash -Algorithm SHA256 $path).Hash.ToLowerInvariant() }

function Get-Asset($repo, $tag, $name) {
    $rel = if ($tag) { "tags/$tag" } else { 'latest' }
    $r = Invoke-RestMethod -Headers $headers "https://api.github.com/repos/$repo/releases/$rel"
    $a = $r.assets | Where-Object name -eq $name
    if (-not $a) { throw "No existe $name en $repo $($r.tag_name)" }
    [pscustomobject]@{ Tag = $r.tag_name; Url = $a.browser_download_url }
}

$lock = if (Test-Path $lockPath) { Get-Content $lockPath -Raw | ConvertFrom-Json } else { $null }
if (-not $Update -and -not $lock) { throw 'Falta tools.lock.json (usa -Update para crearlo).' }

# Windows ARM64 usa la entrada "windows-arm64" del lock; x64 sigue en las claves de la raíz.
$arm64 = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq 'Arm64'
if ($arm64 -and $Update) { throw '-Update solo se admite en Windows x64.' }
$entry = if ($arm64) { $lock.'windows-arm64' } else { $lock }
$specs = @(
    @{ Key = 'ytDlp'; Repo = 'yt-dlp/yt-dlp'; Asset = if ($arm64) { $entry.ytDlp.asset } else { 'yt-dlp_win.zip' }; Dest = 'yt-dlp' },
    @{ Key = 'deno'; Repo = 'denoland/deno'; Asset = if ($arm64) { $entry.deno.asset } else { 'deno-x86_64-pc-windows-msvc.zip' }; Dest = '.' }
)

$tmp = Join-Path ([IO.Path]::GetTempPath()) "xp-music-tools-$([guid]::NewGuid())"
New-Item -ItemType Directory -Force $tmp | Out-Null
New-Item -ItemType Directory -Force $tools | Out-Null
$newLock = [ordered]@{}
try {
    foreach ($s in $specs) {
        $tag = if ($Update) { $null } else { $entry.($s.Key).version }
        $asset = Get-Asset $s.Repo $tag $s.Asset
        $zip = Join-Path $tmp $s.Asset
        Write-Host "Descargando $($s.Asset) $($asset.Tag)…"
        Invoke-WebRequest -Headers $headers $asset.Url -OutFile $zip
        $hash = Get-Sha256 $zip
        if (-not $Update -and $hash -ne $entry.($s.Key).sha256) {
            throw "SHA-256 de $($s.Asset) no coincide con tools.lock.json"
        }
        $dest = Join-Path $tools $s.Dest
        if ($s.Dest -ne '.' -and (Test-Path $dest)) { Remove-Item -Recurse -Force $dest }
        Expand-Archive -Force $zip $dest
        # El zip de yt-dlp para ARM64 trae yt-dlp_arm64.exe; la app espera yt-dlp.exe.
        $armExe = Join-Path $dest 'yt-dlp_arm64.exe'
        if ($s.Key -eq 'ytDlp' -and (Test-Path $armExe)) { Move-Item -Force $armExe (Join-Path $dest 'yt-dlp.exe') }
        $newLock[$s.Key] = [ordered]@{ version = $asset.Tag; asset = $s.Asset; sha256 = $hash }
    }
    # La versión de un solo archivo ya no hace falta: la app usa la "onedir".
    Remove-Item -Force -ErrorAction SilentlyContinue (Join-Path $tools 'yt-dlp.exe')
    if ($Update) {
        # Conserva las entradas de las demás plataformas (linux, macos-*, *-arm64).
        foreach ($p in $lock.PSObject.Properties) { if ($p.Name -notin 'ytDlp', 'deno') { $newLock[$p.Name] = $p.Value } }
        $newLock | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 $lockPath
        Write-Host "tools.lock.json actualizado."
    }
    & (Join-Path $tools 'yt-dlp\yt-dlp.exe') --version
    & (Join-Path $tools 'deno.exe') --version | Select-Object -First 1
} finally {
    Remove-Item -Recurse -Force $tmp
}
