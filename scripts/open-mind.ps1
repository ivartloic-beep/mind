# Ouvrir MIND — mode dev par defaut (code a jour), sinon exe si npm absent.
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

function Find-MindExecutable {
    $names = @("ma-tete.exe", "Ma Tete.exe", "mind.exe")
    $roots = @(
        (Join-Path $Root "src-tauri\target\release"),
        (Join-Path $env:LOCALAPPDATA "Programs"),
        (Join-Path $env:LOCALAPPDATA "com.matete.desktop")
    )

    foreach ($name in $names) {
        foreach ($base in $roots) {
            if (-not (Test-Path $base)) { continue }
            $direct = Join-Path $base $name
            if (Test-Path $direct) { return $direct }
            try {
                $hit = Get-ChildItem -Path $base -Filter $name -Recurse -ErrorAction SilentlyContinue |
                    Select-Object -First 1
                if ($hit) { return $hit.FullName }
            } catch {
            }
        }
    }
    return $null
}

function Start-MindDev {
    $npm = Get-Command npm -ErrorAction SilentlyContinue
    if (-not $npm) { return $false }
    $devScript = Join-Path $Root "scripts\open-mind-dev.ps1"
    if (-not (Test-Path $devScript)) { return $false }
    Write-Host "Lancement mode dev (code Git a jour)..." -ForegroundColor Cyan
    Start-Process -FilePath "powershell.exe" -ArgumentList @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-NoExit",
        "-File", $devScript
    )
    return $true
}

# Preferer le mode dev tant que l'installateur final n'est pas le flux quotidien.
if (Start-MindDev) {
    Start-Sleep -Seconds 2
    exit 0
}

$exe = Find-MindExecutable
if ($exe) {
    Write-Host "Lancement exe: $exe" -ForegroundColor Cyan
    try {
        Start-Process -FilePath $exe -WorkingDirectory (Split-Path $exe -Parent)
        exit 0
    } catch {
        Fail ("Impossible de lancer l exe: " + $_.Exception.Message)
    }
}

Fail "npm introuvable et aucun exe. Installe Node.js 20+ ou build l'exe."
