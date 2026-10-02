# Mettre a jour MIND depuis GitHub (pull + deps) et arreter les anciennes instances.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
if (-not (Test-Path (Join-Path $Root "package.json"))) {
    $Root = "C:\Users\Loic\mind"
}
Set-Location $Root

Write-Host "=== Mise a jour MIND ===" -ForegroundColor Cyan
Write-Host ("Dossier: " + $Root)

foreach ($n in @("ma-tete", "Ma Tete", "Ma Tête", "mind")) {
    Get-Process -Name $n -ErrorAction SilentlyContinue | ForEach-Object {
        Write-Host ("Arret: " + $_.ProcessName) -ForegroundColor Yellow
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    }
}

git pull origin main
if ($LASTEXITCODE -ne 0) { throw "git pull a echoue" }

npm install
if ($LASTEXITCODE -ne 0) { throw "npm install a echoue" }

Write-Host ""
Write-Host "OK — code a jour. Utilise Ouvrir MIND (mode dev)." -ForegroundColor Green
Write-Host "Dans l'app: engrenage Parametres -> Configurer -> Token API." -ForegroundColor Cyan
Write-Host "Appuie sur une touche pour fermer..."
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
