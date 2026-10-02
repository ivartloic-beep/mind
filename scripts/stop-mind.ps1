# Arreter MIND - ferme toutes les instances (exe + desktop:dev).
$ErrorActionPreference = "Continue"

Write-Host "=== Arreter MIND ===" -ForegroundColor Cyan
$stopped = 0

foreach ($n in @("ma-tete", "Ma Tete", "mind")) {
    Get-Process -Name $n -ErrorAction SilentlyContinue | ForEach-Object {
        Write-Host ("Arret: " + $_.ProcessName + " PID " + $_.Id) -ForegroundColor Yellow
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
        $script:stopped++
    }
}

try {
    Get-CimInstance Win32_Process -Filter "Name = 'node.exe'" -ErrorAction SilentlyContinue |
        Where-Object {
            $_.CommandLine -and (
                $_.CommandLine -match 'tauri' -or
                $_.CommandLine -match 'vite' -or
                $_.CommandLine -match 'ma-tete' -or
                $_.CommandLine -match '\\mind\\'
            )
        } |
        ForEach-Object {
            Write-Host ("Arret node PID " + $_.ProcessId) -ForegroundColor Yellow
            Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue
            $script:stopped++
        }
} catch {
}

if ($stopped -eq 0) {
    Write-Host "Aucune instance MIND detectee." -ForegroundColor Green
} else {
    Write-Host ("MIND arrete (" + $stopped + " process).") -ForegroundColor Green
}

Start-Sleep -Seconds 1
