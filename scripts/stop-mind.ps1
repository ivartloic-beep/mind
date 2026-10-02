# Arreter MIND — ferme toutes les instances (exe + tauri/dev).
$ErrorActionPreference = "Continue"

Write-Host "=== Arreter MIND ===" -ForegroundColor Cyan

$stopped = 0
$names = @("ma-tete", "Ma Tete", "Ma Tête", "mind", "node")

# Processus app connus
foreach ($n in @("ma-tete", "Ma Tete", "Ma Tête", "mind")) {
    Get-Process -Name $n -ErrorAction SilentlyContinue | ForEach-Object {
        Write-Host ("Arret: " + $_.ProcessName + " PID " + $_.Id) -ForegroundColor Yellow
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
        $script:stopped++
    }
}

# Mode desktop:dev : vite / tauri souvent sous node avec cwd = mind
$mindRoot = "C:\Users\Loic\mind"
try {
    Get-CimInstance Win32_Process -Filter "Name = 'node.exe'" -ErrorAction SilentlyContinue |
        Where-Object {
            $_.CommandLine -and (
                $_.CommandLine -match 'tauri' -or
                $_.CommandLine -match 'vite' -or
                $_.CommandLine -match [regex]::Escape($mindRoot)
            )
        } |
        ForEach-Object {
            Write-Host ("Arret node: PID " + $_.ProcessId) -ForegroundColor Yellow
            Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue
            $script:stopped++
        }
} catch {
    # fallback: rien
}

# Rust/cargo tauri parfois
foreach ($n in @("cargo", "rustc")) {
    Get-Process -Name $n -ErrorAction SilentlyContinue | ForEach-Object {
        try {
            $cmd = (Get-CimInstance Win32_Process -Filter ("ProcessId=" + $_.Id) -ErrorAction SilentlyContinue).CommandLine
            if ($cmd -and ($cmd -match 'tauri' -or $cmd -match 'ma-tete')) {
                Write-Host ("Arret: " + $_.ProcessName + " PID " + $_.Id) -ForegroundColor Yellow
                Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
                $script:stopped++
            }
        } catch {
        }
    }
}

if ($stopped -eq 0) {
    Write-Host "Aucune instance MIND detectee." -ForegroundColor Green
} else {
    Write-Host ("MIND arrete (" + $stopped + " process).") -ForegroundColor Green
}

Start-Sleep -Seconds 1
