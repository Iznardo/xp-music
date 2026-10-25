# Downloads yt-dlp ("onedir" version) and deno into tools/, at the versions pinned in
# tools.lock.json, and checks their SHA-256. With no arguments it uses the lock; with
# -Update it takes the latest published versions and rewrites the lock.
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
    if (-not $a) { throw "$name does not exist in $repo $($r.tag_name)" }
    [pscustomobject]@{ Tag = $r.tag_name; Url = $a.browser_download_url }
}

$lock = if (Test-Path $lockPath) { Get-Content $lockPath -Raw | ConvertFrom-Json } else { $null }
if (-not $Update -and -not $lock) { throw 'tools.lock.json is missing (use -Update to create it).' }

# Windows ARM64 uses the "windows-arm64" lock entry; x64 stays on the root keys.
$arm64 = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq 'Arm64'
if ($arm64 -and $Update) { throw '-Update is only supported on Windows x64.' }
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
        Write-Host "Downloading $($s.Asset) $($asset.Tag)…"
        Invoke-WebRequest -Headers $headers $asset.Url -OutFile $zip
        $hash = Get-Sha256 $zip
        if (-not $Update -and $hash -ne $entry.($s.Key).sha256) {
            throw "SHA-256 of $($s.Asset) does not match tools.lock.json"
        }
        $dest = Join-Path $tools $s.Dest
        if ($s.Dest -ne '.' -and (Test-Path $dest)) { Remove-Item -Recurse -Force $dest }
        Expand-Archive -Force $zip $dest
        # The ARM64 yt-dlp zip contains yt-dlp_arm64.exe; the app expects yt-dlp.exe.
        $armExe = Join-Path $dest 'yt-dlp_arm64.exe'
        if ($s.Key -eq 'ytDlp' -and (Test-Path $armExe)) { Move-Item -Force $armExe (Join-Path $dest 'yt-dlp.exe') }
        $newLock[$s.Key] = [ordered]@{ version = $asset.Tag; asset = $s.Asset; sha256 = $hash }
    }
    # The single-file version is no longer needed: the app uses the "onedir" one.
    Remove-Item -Force -ErrorAction SilentlyContinue (Join-Path $tools 'yt-dlp.exe')
    if ($Update) {
        # Keep the entries of the other platforms (linux, macos-*, *-arm64).
        foreach ($p in $lock.PSObject.Properties) { if ($p.Name -notin 'ytDlp', 'deno') { $newLock[$p.Name] = $p.Value } }
        $newLock | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 $lockPath
        Write-Host "tools.lock.json updated."
    }
    & (Join-Path $tools 'yt-dlp\yt-dlp.exe') --version
    & (Join-Path $tools 'deno.exe') --version | Select-Object -First 1
} finally {
    Remove-Item -Recurse -Force $tmp
}
