# Mettre a jour MIND depuis GitHub (pull + deps).
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
if (-not (Test-Path (Join-Path $Root "package.json"))) {
    $Root = "C:\Users\Loic\mind"
}
Set-Location $Root

Write-Host "=== Mise a jour MIND ===" -ForegroundColor Cyan
Write-Host "Dossier: $Root"

git pull origin main
if ($LASTEXITCODE -ne 0) { throw "git pull a echoue" }

npm install
if ($LASTEXITCODE -ne 0) { throw "npm install a echoue" }

Write-Host ""
Write-Host "OK — code a jour. Lance « Ouvrir MIND » pour demarrer." -ForegroundColor Green
Write-Host "Appuie sur une touche pour fermer..."
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
