# Cree 2 raccourcis sur le Bureau : Mettre a jour MIND / Ouvrir MIND
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
if (-not (Test-Path (Join-Path $Root "package.json"))) {
    $Root = "C:\Users\Loic\mind"
}

$Desktop = [Environment]::GetFolderPath("Desktop")
$UpdatePs1 = Join-Path $Root "scripts\update-mind.ps1"
$OpenPs1 = Join-Path $Root "scripts\open-mind.ps1"

foreach ($p in @($UpdatePs1, $OpenPs1)) {
    if (-not (Test-Path $p)) { throw "Script introuvable: $p" }
}

$Wsh = New-Object -ComObject WScript.Shell

function New-MindShortcut {
    param(
        [string]$Name,
        [string]$ScriptPath,
        [string]$IconHint
    )
    $lnkPath = Join-Path $Desktop "$Name.lnk"
    $sc = $Wsh.CreateShortcut($lnkPath)
    $sc.TargetPath = "powershell.exe"
    # Ouvrir MIND : garder la fenetre si erreur ; update peut se fermer apres succes.
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

Write-Host ""
Write-Host "Raccourcis crees sur le Bureau." -ForegroundColor Cyan
Write-Host "Appuie sur une touche pour fermer..."
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
