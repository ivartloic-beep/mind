# Ouvrir MIND — exe installe / release, sinon mode dev.
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
        (Join-Path $Root "src-tauri\target\debug"),
        (Join-Path $env:LOCALAPPDATA "Programs"),
        (Join-Path $env:LOCALAPPDATA "com.matete.desktop"),
        $env:LOCALAPPDATA
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

    try {
        $any = Get-ChildItem -Path (Join-Path $env:LOCALAPPDATA "Programs") -Filter "ma-tete.exe" -Recurse -ErrorAction SilentlyContinue |
            Select-Object -First 1
        if ($any) { return $any.FullName }
    } catch {
    }

    return $null
}

$exe = Find-MindExecutable
if ($exe) {
    Write-Host "Lancement: $exe" -ForegroundColor Cyan
    try {
        Start-Process -FilePath $exe -WorkingDirectory (Split-Path $exe -Parent)
        exit 0
    } catch {
        Fail ("Impossible de lancer l exe: " + $_.Exception.Message)
    }
}

Write-Host "Aucun exe trouve - mode dev (npm run desktop:dev)." -ForegroundColor Yellow
Write-Host ("Dossier: " + $Root)

$npm = Get-Command npm -ErrorAction SilentlyContinue
if (-not $npm) {
    Fail "npm introuvable. Installe Node.js 20+ puis relance Ouvrir MIND."
}

$devScript = Join-Path $Root "scripts\open-mind-dev.ps1"
if (-not (Test-Path $devScript)) {
    Fail ("Script dev introuvable: " + $devScript)
}

Start-Process -FilePath "powershell.exe" -ArgumentList @(
    "-NoProfile",
    "-ExecutionPolicy", "Bypass",
    "-NoExit",
    "-File", $devScript
)
Write-Host "Fenetre dev lancee." -ForegroundColor Green
Start-Sleep -Seconds 2
