# Lance MIND en mode dev (appele par open-mind.ps1).
$ErrorActionPreference = "Continue"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $Root
Write-Host "MIND dev — $Root" -ForegroundColor Cyan
npm run desktop:dev
