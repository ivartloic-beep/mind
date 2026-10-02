# Mettre a jour MIND depuis GitHub (pull + deps) et arreter les anciennes instances.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
if (-not (Test-Path (Join-Path $Root "package.json"))) {
    $Root = "C:\Users\Loic\mind"
}
Set-Location $Root

Write-Host "=== Mise a jour MIND ===" -ForegroundColor Cyan
Write-Host ("Dossier: " + $Root)

foreach ($n in @("ma-tete", "Ma Tete", "mind")) {
    Get-Process -Name $n -ErrorAction SilentlyContinue | ForEach-Object {
        Write-Host ("Arret: " + $_.ProcessName) -ForegroundColor Yellow
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    }
}

git fetch origin main
if ($LASTEXITCODE -ne 0) { throw "git fetch a echoue" }
git checkout main
if ($LASTEXITCODE -ne 0) { throw "git checkout main a echoue" }
git pull --ff-only origin main
if ($LASTEXITCODE -ne 0) { throw "git pull a echoue" }

$commit = (git rev-parse --short HEAD)
$subject = (git log -1 --pretty=format:"%s")
Write-Host ("Commit: " + $commit + " - " + $subject) -ForegroundColor Yellow

npm install
if ($LASTEXITCODE -ne 0) { throw "npm install a echoue" }

Write-Host ""
Write-Host "OK - code a jour. Utilise Ouvrir MIND (mode dev)." -ForegroundColor Green
Write-Host "Gestion: Parametres (engrenage) -> URL API Gestion -> Se connecter -> Importer taches locales (si besoin)." -ForegroundColor Cyan
Write-Host "Cloud mind.louetline.fr (optionnel): Parametres -> Cloud -> Token API." -ForegroundColor Cyan
Write-Host "Appuie sur une touche pour fermer..."
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
