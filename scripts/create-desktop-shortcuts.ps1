# Cree les raccourcis Bureau : Mettre a jour / Ouvrir / Arreter MIND
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
if (-not (Test-Path (Join-Path $Root "package.json"))) {
    $Root = "C:\Users\Loic\mind"
}

$Desktop = [Environment]::GetFolderPath("Desktop")
$UpdatePs1 = Join-Path $Root "scripts\update-mind.ps1"
$OpenPs1 = Join-Path $Root "scripts\open-mind.ps1"
$StopPs1 = Join-Path $Root "scripts\stop-mind.ps1"

foreach ($p in @($UpdatePs1, $OpenPs1, $StopPs1)) {
    if (-not (Test-Path $p)) { throw "Script introuvable: $p" }
}

$Wsh = New-Object -ComObject WScript.Shell

function New-MindShortcut {
    param(
        [string]$Name,
        [string]$ScriptPath
    )
    $lnkPath = Join-Path $Desktop "$Name.lnk"
    $sc = $Wsh.CreateShortcut($lnkPath)
    $sc.TargetPath = "powershell.exe"
    if ($Name -like "*Ouvrir*") {
        $sc.Arguments = "-NoProfile -ExecutionPolicy Bypass -NoExit -File `"$ScriptPath`""
    } else {
        $sc.Arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$ScriptPath`""
    }
    $sc.WorkingDirectory = $Root
    $sc.WindowStyle = 1
    $sc.Description = $Name
    $ico = Join-Path $Root "src-tauri\icons\icon.ico"
    if (Test-Path $ico) { $sc.IconLocation = $ico }
    $sc.Save()
    Write-Host "Cree: $lnkPath" -ForegroundColor Green
}

New-MindShortcut -Name "Mettre a jour MIND" -ScriptPath $UpdatePs1
New-MindShortcut -Name "Ouvrir MIND" -ScriptPath $OpenPs1
New-MindShortcut -Name "Arreter MIND" -ScriptPath $StopPs1

Write-Host ""
Write-Host "Raccourcis crees sur le Bureau." -ForegroundColor Cyan
Write-Host "Appuie sur une touche pour fermer..."
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
