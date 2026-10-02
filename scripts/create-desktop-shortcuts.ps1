# Cree les raccourcis Bureau : Mettre a jour / Ouvrir / Arreter MIND
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
if (-not (Test-Path (Join-Path $Root "package.json"))) {
    $Root = "C:\Users\Loic\mind"
}

$Desktop = [Environment]::GetFolderPath("Desktop")
$UpdatePs1 = Join-Path $Root "scripts\update-mind.ps1"
$OpenVbs = Join-Path $Root "scripts\open-mind.vbs"
$StopVbs = Join-Path $Root "scripts\stop-mind.vbs"

foreach ($p in @($UpdatePs1, $OpenVbs, $StopVbs)) {
    if (-not (Test-Path $p)) { throw "Script introuvable: $p" }
}

$Wsh = New-Object -ComObject WScript.Shell
$ico = Join-Path $Root "src-tauri\icons\icon.ico"

function New-MindShortcut {
    param(
        [string]$Name,
        [string]$TargetPath,
        [string]$Arguments,
        [int]$WindowStyle = 1
    )
    $lnkPath = Join-Path $Desktop "$Name.lnk"
    $sc = $Wsh.CreateShortcut($lnkPath)
    $sc.TargetPath = $TargetPath
    $sc.Arguments = $Arguments
    $sc.WorkingDirectory = $Root
    $sc.WindowStyle = $WindowStyle
    $sc.Description = $Name
    if (Test-Path $ico) { $sc.IconLocation = $ico }
    $sc.Save()
    Write-Host "Cree: $lnkPath" -ForegroundColor Green
}

New-MindShortcut -Name "Mettre a jour MIND" `
    -TargetPath "powershell.exe" `
    -Arguments ("-NoProfile -ExecutionPolicy Bypass -File `"" + $UpdatePs1 + "`"")

New-MindShortcut -Name "Ouvrir MIND" `
    -TargetPath "wscript.exe" `
    -Arguments ("//nologo `"" + $OpenVbs + "`"") `
    -WindowStyle 7

New-MindShortcut -Name "Arreter MIND" `
    -TargetPath "wscript.exe" `
    -Arguments ("//nologo `"" + $StopVbs + "`"") `
    -WindowStyle 7

Write-Host ""
Write-Host "Raccourcis crees sur le Bureau." -ForegroundColor Cyan
Write-Host "Appuie sur une touche pour fermer..."
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
