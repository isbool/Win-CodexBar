# CodexBar Windows Installer
# Run: powershell -ExecutionPolicy Bypass -File install.ps1

$ErrorActionPreference = "Stop"

Write-Host "================================" -ForegroundColor Cyan
Write-Host "  CodexBar Installer v1.0.0" -ForegroundColor Cyan
Write-Host "================================" -ForegroundColor Cyan
Write-Host ""

# Determine install location
$installDir = "$env:LOCALAPPDATA\CodexBar"
$exePath = "$installDir\codexbar.exe"

# Find the source exe
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$sourceExe = Join-Path $scriptDir "rust\target\release\codexbar.exe"

if (-not (Test-Path $sourceExe)) {
    # Try current directory
    $sourceExe = Join-Path $scriptDir "codexbar.exe"
}

if (-not (Test-Path $sourceExe)) {
    Write-Host "ERROR: codexbar.exe not found!" -ForegroundColor Red
    Write-Host "Make sure you run this from the Win-CodexBar directory or place codexbar.exe next to this script."
    exit 1
}

Write-Host "Source: $sourceExe"
Write-Host "Install to: $installDir"
Write-Host ""

# Create install directory
Write-Host "[1/4] Creating install directory..." -ForegroundColor Yellow
if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

# Copy executable
Write-Host "[2/4] Copying codexbar.exe..." -ForegroundColor Yellow
Copy-Item $sourceExe $exePath -Force

# Add to PATH
Write-Host "[3/4] Adding to PATH..." -ForegroundColor Yellow
$currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($currentPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$installDir", "User")
    Write-Host "       Added $installDir to PATH" -ForegroundColor Green
} else {
    Write-Host "       Already in PATH" -ForegroundColor Green
}

# Create Start Menu shortcut
Write-Host "[4/4] Creating Start Menu shortcut..." -ForegroundColor Yellow
$startMenuDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs"
$shortcutPath = "$startMenuDir\CodexBar.lnk"

$WshShell = New-Object -ComObject WScript.Shell
$shortcut = $WshShell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = $exePath
$shortcut.Arguments = "menubar"
$shortcut.Description = "AI Provider Usage Monitor"
$shortcut.Save()

Write-Host ""
Write-Host "================================" -ForegroundColor Green
Write-Host "  Installation Complete!" -ForegroundColor Green
Write-Host "================================" -ForegroundColor Green
Write-Host ""
Write-Host "To start CodexBar:"
Write-Host "  1. Open a NEW terminal (to refresh PATH)"
Write-Host "  2. Run: codexbar menubar"
Write-Host ""
Write-Host "Or search 'CodexBar' in Start Menu"
Write-Host ""

# Ask to start now
$response = Read-Host "Start CodexBar now? (Y/n)"
if ($response -ne "n" -and $response -ne "N") {
    Start-Process $exePath -ArgumentList "menubar"
    Write-Host "CodexBar started! Check your system tray." -ForegroundColor Cyan
}
