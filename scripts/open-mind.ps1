# Ouvrir MIND - toujours le code Git via desktop:dev (pas l'ancien exe).
$ErrorActionPreference = "Continue"

function Wait-Key {
    Write-Host ""
    Write-Host "Appuie sur une touche pour fermer..."
    $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
}

function Fail([string]$Message) {
    Write-Host $Message -ForegroundColor Red
    Wait-Key
    exit 1
}

$machinePath = [System.Environment]::GetEnvironmentVariable("Path", "Machine")
$userPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
if ($machinePath -and $userPath) {
    $env:Path = "$machinePath;$userPath"
} elseif ($machinePath) {
    $env:Path = $machinePath
}

$Root = Split-Path -Parent $PSScriptRoot
if (-not (Test-Path (Join-Path $Root "package.json"))) {
    $Root = "C:\Users\Loic\mind"
}
if (-not (Test-Path (Join-Path $Root "package.json"))) {
    Fail "Dossier projet introuvable: C:\Users\Loic\mind"
}
Set-Location -LiteralPath $Root

foreach ($n in @("ma-tete", "Ma Tete", "mind")) {
    Get-Process -Name $n -ErrorAction SilentlyContinue | ForEach-Object {
        Write-Host ("Arret process: " + $_.ProcessName + " PID " + $_.Id) -ForegroundColor Yellow
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    }
}

$npm = Get-Command npm -ErrorAction SilentlyContinue
if (-not $npm) {
    Fail "npm introuvable. Installe Node.js 20+."
}

$devScript = Join-Path $Root "scripts\open-mind-dev.ps1"
if (-not (Test-Path $devScript)) {
    Fail ("Script manquant: " + $devScript + " - fais git pull origin main")
}

Write-Host "Lancement MIND (code Git a jour, desktop:dev)..." -ForegroundColor Cyan
Write-Host ("Dossier: " + $Root)
Start-Process -FilePath "powershell.exe" -ArgumentList @(
    "-NoProfile",
    "-ExecutionPolicy", "Bypass",
    "-NoExit",
    "-File", $devScript
)
Start-Sleep -Seconds 2
