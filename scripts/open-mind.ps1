# Ouvrir MIND - lance desktop:dev sans fenetre console.
$ErrorActionPreference = "Continue"

function Fail([string]$Message) {
    $log = Join-Path $env:TEMP "mind-open-error.txt"
    Set-Content -Path $log -Value $Message -Encoding ascii
    Start-Process -FilePath "powershell.exe" -ArgumentList @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-Command",
        ("Add-Type -AssemblyName PresentationFramework; [System.Windows.MessageBox]::Show('" + $Message.Replace("'","''") + "','MIND')")
    ) | Out-Null
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

Start-Process -FilePath "powershell.exe" -WindowStyle Hidden -ArgumentList @(
    "-NoProfile",
    "-ExecutionPolicy", "Bypass",
    "-File", $devScript
) | Out-Null
