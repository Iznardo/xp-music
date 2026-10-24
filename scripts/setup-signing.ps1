# One-time code-signing setup (specs/007, FR-007-03).
#
# Creates a self-signed "CN=iznardo" code-signing certificate with a NON-exportable
# key in your user store and marks it as trusted ONLY on this machine and user
# (CurrentUser\Root and CurrentUser\TrustedPublisher). Windows will show a security
# warning when adding it to "Root": you have to accept it.
#
# Saves the thumbprint in src-tauri\signing.local.json (ignored by git), which
# scripts\release.ps1 reads. If a valid certificate already exists, it is reused.
#
#   powershell -ExecutionPolicy Bypass -File scripts\setup-signing.ps1
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root 'src-tauri\signing.local.json'
$subject = 'CN=iznardo'

$cert = Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert |
    Where-Object { $_.Subject -eq $subject -and $_.NotAfter -gt (Get-Date).AddDays(30) -and $_.HasPrivateKey } |
    Sort-Object NotAfter -Descending | Select-Object -First 1

if ($cert) {
    Write-Host "Reusing existing certificate $($cert.Thumbprint) (expires $($cert.NotAfter.ToShortDateString()))."
} else {
    $cert = New-SelfSignedCertificate -Type CodeSigningCert -Subject $subject `
        -KeyAlgorithm RSA -KeyLength 3072 -HashAlgorithm SHA256 `
        -KeyExportPolicy NonExportable -NotAfter (Get-Date).AddYears(5) `
        -CertStoreLocation Cert:\CurrentUser\My
    Write-Host "Certificate created: $($cert.Thumbprint)"
}

$cer = Join-Path ([IO.Path]::GetTempPath()) 'xp-music-signing.cer'
Export-Certificate -Cert $cert -FilePath $cer | Out-Null
try {
    foreach ($store in 'Root', 'TrustedPublisher') {
        $exists = Get-ChildItem "Cert:\CurrentUser\$store" | Where-Object Thumbprint -eq $cert.Thumbprint
        if (-not $exists) {
            Import-Certificate -FilePath $cer -CertStoreLocation "Cert:\CurrentUser\$store" | Out-Null
            Write-Host "Added to CurrentUser\$store."
        }
    }
} finally {
    Remove-Item -Force $cer
}

[ordered]@{ certificateThumbprint = $cert.Thumbprint; subject = $subject } |
    ConvertTo-Json | Set-Content -Encoding utf8 $out
Write-Host "Thumbprint saved to $out"

# signtool ships with the Windows SDK; warn if it is missing.
$signtool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" -ErrorAction SilentlyContinue |
    Select-Object -Last 1
if ($signtool) { Write-Host "signtool: $($signtool.FullName)" }
else { Write-Warning 'signtool.exe not found: install the "Windows SDK" (winget install Microsoft.WindowsSDK.10.0.26100).' }
