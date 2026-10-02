# Arreter MIND - ferme app + node + cmd/powershell lies a MIND.
$ErrorActionPreference = "Continue"

Write-Host "=== Arreter MIND ===" -ForegroundColor Cyan
$stopped = 0
$selfPid = $PID

function Stop-MindProcess([int]$ProcessId, [string]$Label) {
    if ($ProcessId -le 0) { return }
    if ($ProcessId -eq $selfPid) { return }
    try {
        Write-Host ("Arret: " + $Label + " PID " + $ProcessId) -ForegroundColor Yellow
        Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
        $script:stopped++
    } catch {
    }
}

function Command-LooksLikeMind([string]$CommandLine) {
    if (-not $CommandLine) { return $false }
    return (
        $CommandLine -match 'tauri' -or
        $CommandLine -match 'vite' -or
        $CommandLine -match 'ma-tete' -or
        $CommandLine -match 'desktop:dev' -or
        $CommandLine -match 'open-mind' -or
        $CommandLine -match 'stop-mind' -or
        $CommandLine -match 'update-mind' -or
        $CommandLine -match '\\mind\\' -or
        $CommandLine -match '/mind/' -or
        $CommandLine -match 'mind-dev\.log'
    )
}

foreach ($n in @("ma-tete", "Ma Tete", "mind")) {
    Get-Process -Name $n -ErrorAction SilentlyContinue | ForEach-Object {
        Stop-MindProcess -ProcessId $_.Id -Label $_.ProcessName
    }
}

$procNames = @("node.exe", "npm.cmd", "cmd.exe", "powershell.exe", "pwsh.exe", "conhost.exe")
try {
    Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
        Where-Object {
            ($procNames -contains $_.Name) -and (Command-LooksLikeMind $_.CommandLine)
        } |
        ForEach-Object {
            # Ne pas tuer ce script stop-mind lui-meme.
            if ($_.ProcessId -eq $selfPid) { return }
            if ($_.CommandLine -and $_.CommandLine -match 'stop-mind\.ps1' -and $_.ProcessId -eq $selfPid) { return }
            Stop-MindProcess -ProcessId $_.ProcessId -Label $_.Name
        }
} catch {
}

# Second passage: parents cmd/powershell qui ont encore mind dans la ligne de commande.
try {
    Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
        Where-Object {
            ($_.Name -eq 'cmd.exe' -or $_.Name -eq 'powershell.exe' -or $_.Name -eq 'pwsh.exe') -and
            (Command-LooksLikeMind $_.CommandLine) -and
            ($_.ProcessId -ne $selfPid)
        } |
        ForEach-Object {
            Stop-MindProcess -ProcessId $_.ProcessId -Label $_.Name
        }
} catch {
}

if ($stopped -eq 0) {
    Write-Host "Aucune instance MIND detectee." -ForegroundColor Green
} else {
    Write-Host ("MIND arrete (" + $stopped + " process).") -ForegroundColor Green
}

Start-Sleep -Seconds 1
