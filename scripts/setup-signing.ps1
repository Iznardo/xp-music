# Configuración única de la firma de código (specs/007, FR-007-03).
#
# Crea un certificado de firma de código autofirmado "CN=iznardo" con clave NO
# exportable en tu almacén de usuario y lo marca como de confianza SOLO en este
# equipo y usuario (CurrentUser\Root y CurrentUser\TrustedPublisher). Windows
# mostrará un aviso de seguridad al añadirlo a "Root": hay que aceptarlo.
#
# Guarda la huella en src-tauri\signing.local.json (ignorado por git), que lee
# scripts\release.ps1. Si ya existe un certificado válido, lo reutiliza.
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
    Write-Host "Se reutiliza el certificado existente $($cert.Thumbprint) (caduca $($cert.NotAfter.ToShortDateString()))."
} else {
    $cert = New-SelfSignedCertificate -Type CodeSigningCert -Subject $subject `
        -KeyAlgorithm RSA -KeyLength 3072 -HashAlgorithm SHA256 `
        -KeyExportPolicy NonExportable -NotAfter (Get-Date).AddYears(5) `
        -CertStoreLocation Cert:\CurrentUser\My
    Write-Host "Certificado creado: $($cert.Thumbprint)"
}

$cer = Join-Path ([IO.Path]::GetTempPath()) 'xp-music-signing.cer'
Export-Certificate -Cert $cert -FilePath $cer | Out-Null
try {
    foreach ($store in 'Root', 'TrustedPublisher') {
        $exists = Get-ChildItem "Cert:\CurrentUser\$store" | Where-Object Thumbprint -eq $cert.Thumbprint
        if (-not $exists) {
            Import-Certificate -FilePath $cer -CertStoreLocation "Cert:\CurrentUser\$store" | Out-Null
            Write-Host "Añadido a CurrentUser\$store."
        }
    }
} finally {
    Remove-Item -Force $cer
}

[ordered]@{ certificateThumbprint = $cert.Thumbprint; subject = $subject } |
    ConvertTo-Json | Set-Content -Encoding utf8 $out
Write-Host "Huella guardada en $out"

# signtool viene con el Windows SDK; avisar si no está.
$signtool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" -ErrorAction SilentlyContinue |
    Select-Object -Last 1
if ($signtool) { Write-Host "signtool: $($signtool.FullName)" }
else { Write-Warning 'No se encuentra signtool.exe: instala el "Windows SDK" (winget install Microsoft.WindowsSDK.10.0.26100).' }
