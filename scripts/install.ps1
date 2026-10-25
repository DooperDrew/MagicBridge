<#
.SYNOPSIS
    Installs MagicBridge for the current user without requiring Administrator privileges.

.DESCRIPTION
    Installs MagicBridge into %LOCALAPPDATA%\Programs\MagicBridge, creates Start Menu
    shortcuts, optionally configures autostart on Windows logon (HKCU), and launches
    the background agent.

.PARAMETER TargetDir
    Installation directory. Defaults to "$env:LOCALAPPDATA\Programs\MagicBridge".

.PARAMETER EnableAutostart
    Configures MagicBridge to launch automatically on Windows login (per-user HKCU). Default is $true.

.PARAMETER LaunchNow
    Starts the MagicBridge background agent immediately after installation. Default is $true.

.EXAMPLE
    .\install.ps1
    .\install.ps1 -TargetDir "C:\Tools\MagicBridge" -EnableAutostart:$false
#>

[CmdletBinding()]
param (
    [string]$TargetDir = "$env:LOCALAPPDATA\Programs\MagicBridge",
    [switch]$EnableAutostart = $true,
    [switch]$LaunchNow = $true
)

$ErrorActionPreference = "Stop"

Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "         MagicBridge User-Space Installer         " -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan

# Source files location (either repository root or unpacked release archive)
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition

# Locate agent binary
$CandidateAgentPaths = @(
    Join-Path $ScriptDir "magicbridge-agent.exe",
    Join-Path $ScriptDir "..\target\release\magicbridge-agent.exe"
)
$SourceAgent = $CandidateAgentPaths | Where-Object { Test-Path $_ } | Select-Object -First 1

if (-not $SourceAgent) {
    throw "magicbridge-agent.exe not found in $ScriptDir or target\release. Please run package-release.ps1 or build the project first."
}
$SourceDir = Split-Path -Parent $SourceAgent

Write-Host "Source directory: $SourceDir"
Write-Host "Target directory: $TargetDir"

# Stop running instances if any
Write-Host "`nStopping any existing MagicBridge processes..." -ForegroundColor Yellow
Get-Process -Name "magicbridge-agent", "magicbridge-settings" -ErrorAction SilentlyContinue | ForEach-Object {
    Write-Host "  Stopping $($_.ProcessName) (PID $($_.Id))..."
    Stop-Process -Id $_.Id -Force
}
Start-Sleep -Milliseconds 500

# Create destination directory
if (-not (Test-Path $TargetDir)) {
    New-Item -ItemType Directory -Path $TargetDir -Force | Out-Null
}

# Copy executables
Write-Host "Copying application binaries..." -ForegroundColor Cyan
Copy-Item (Join-Path $SourceDir "magicbridge-agent.exe") -Destination $TargetDir -Force
$SettingsExe = Join-Path $SourceDir "magicbridge-settings.exe"
if (Test-Path $SettingsExe) {
    Copy-Item $SettingsExe -Destination $TargetDir -Force
}

# Copy locales
$TargetLocales = Join-Path $TargetDir "locales"
if (-not (Test-Path $TargetLocales)) {
    New-Item -ItemType Directory -Path $TargetLocales -Force | Out-Null
}

$LocalesSource = Join-Path $SourceDir "locales"
if (-not (Test-Path $LocalesSource)) {
    $LocalesSource = Join-Path $ScriptDir "..\crates\settings\locales"
}
if (Test-Path $LocalesSource) {
    Copy-Item (Join-Path $LocalesSource "*.toml") -Destination $TargetLocales -Force
}

# Copy documentation and uninstaller
@("README.md", "LICENSE", "user-guide.md", "docs\user-guide.md") | ForEach-Object {
    $DocPath = Join-Path $SourceDir $_
    if (-not (Test-Path $DocPath)) {
        $DocPath = Join-Path $ScriptDir "..\\$_"
    }
    if (Test-Path $DocPath) {
        Copy-Item $DocPath -Destination $TargetDir -Force
    }
}

$UninstallScript = Join-Path $ScriptDir "uninstall.ps1"
if (Test-Path $UninstallScript) {
    Copy-Item $UninstallScript -Destination $TargetDir -Force
}

# Create Start Menu Shortcuts
Write-Host "Creating Start Menu shortcuts..." -ForegroundColor Cyan
$StartMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\MagicBridge"
if (-not (Test-Path $StartMenuDir)) {
    New-Item -ItemType Directory -Path $StartMenuDir -Force | Out-Null
}

$WshShell = New-Object -ComObject WScript.Shell

# Shortcut to Settings
$SettingsShortcut = $WshShell.CreateShortcut((Join-Path $StartMenuDir "MagicBridge Settings.lnk"))
$SettingsShortcut.TargetPath = Join-Path $TargetDir "magicbridge-settings.exe"
$SettingsShortcut.WorkingDirectory = $TargetDir
$SettingsShortcut.Description = "Configure Apple Magic Keyboard mappings and profiles."
$SettingsShortcut.Save()

# Shortcut to Agent
$AgentShortcut = $WshShell.CreateShortcut((Join-Path $StartMenuDir "MagicBridge Agent.lnk"))
$AgentShortcut.TargetPath = Join-Path $TargetDir "magicbridge-agent.exe"
$AgentShortcut.WorkingDirectory = $TargetDir
$AgentShortcut.Description = "Background keyboard remap hook and battery monitor."
$AgentShortcut.Save()

# Configure autostart in registry
if ($EnableAutostart) {
    Write-Host "Configuring autostart in HKCU Run registry..." -ForegroundColor Cyan
    $RunKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
    $AgentBinaryPath = "`"$TargetDir\magicbridge-agent.exe`""
    Set-ItemProperty -Path $RunKey -Name "MagicBridge" -Value $AgentBinaryPath
    Write-Host "  Autostart registered: $AgentBinaryPath" -ForegroundColor Green
}

# Launch agent if requested
if ($LaunchNow) {
    Write-Host "Launching MagicBridge background agent..." -ForegroundColor Cyan
    Start-Process -FilePath (Join-Path $TargetDir "magicbridge-agent.exe") -WorkingDirectory $TargetDir
}

Write-Host "`n==================================================" -ForegroundColor Green
Write-Host "       MAGICBRIDGE INSTALLATION COMPLETED         " -ForegroundColor Green
Write-Host "==================================================" -ForegroundColor Green
Write-Host "Installation path: $TargetDir"
Write-Host "Configuration dir: $env:APPDATA\MagicBridge"
Write-Host "Start Menu:        $StartMenuDir"
Write-Host ""
