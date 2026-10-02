# Ouvrir MIND (exe release si present, sinon mode dev).
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
if (-not (Test-Path (Join-Path $Root "package.json"))) {
    $Root = "C:\Users\Loic\mind"
}
Set-Location $Root

$ReleaseExe = Join-Path $Root "src-tauri\target\release\ma-tete.exe"
$Installed = @(
    (Join-Path $env:LOCALAPPDATA "Ma Tete\ma-tete.exe"),
    (Join-Path $env:LOCALAPPDATA "Programs\Ma Tete\ma-tete.exe")
) | Where-Object { Test-Path $_ } | Select-Object -First 1

if ($Installed) {
    Write-Host "Lancement installateur: $Installed" -ForegroundColor Cyan
    Start-Process -FilePath $Installed
    exit 0
}

if (Test-Path $ReleaseExe) {
    Write-Host "Lancement release: $ReleaseExe" -ForegroundColor Cyan
    Start-Process -FilePath $ReleaseExe
    exit 0
}

Write-Host "Pas d'exe trouve — demarrage en mode dev (terminal visible)." -ForegroundColor Yellow
npm run desktop:dev
