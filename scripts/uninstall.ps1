<#
.SYNOPSIS
    Uninstalls MagicBridge, removes autostart entries, and removes application files.

.DESCRIPTION
    Stops running MagicBridge processes, removes the autostart entry from the
    current user registry (HKCU\Software\Microsoft\Windows\CurrentVersion\Run),
    deletes Start Menu shortcuts, and optionally purges configuration data
    in %APPDATA%\MagicBridge.

.PARAMETER TargetDir
    The installation directory to remove. Defaults to "$env:LOCALAPPDATA\Programs\MagicBridge".

.PARAMETER PurgeData
    If specified, permanently removes user settings and logs in %APPDATA%\MagicBridge.

.EXAMPLE
    .\uninstall.ps1
    .\uninstall.ps1 -PurgeData
#>

[CmdletBinding()]
param (
    [string]$TargetDir = "$env:LOCALAPPDATA\Programs\MagicBridge",
    [switch]$PurgeData
)

$ErrorActionPreference = "Stop"

Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "         MagicBridge Uninstallation Tool          " -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan

# 1. Stop running processes
Write-Host "`n[1/4] Stopping running MagicBridge processes..." -ForegroundColor Cyan
Get-Process -Name "magicbridge-agent", "magicbridge-settings" -ErrorAction SilentlyContinue | ForEach-Object {
    Write-Host "  Stopping $($_.ProcessName) (PID $($_.Id))..."
    Stop-Process -Id $_.Id -Force
}
Start-Sleep -Milliseconds 500

# 2. Remove autostart entry from registry
Write-Host "`n[2/4] Removing autostart entry from HKCU Run..." -ForegroundColor Cyan
$RunKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
try {
    $existing = Get-ItemProperty -Path $RunKey -Name "MagicBridge" -ErrorAction SilentlyContinue
    if ($existing) {
        Remove-ItemProperty -Path $RunKey -Name "MagicBridge" -ErrorAction SilentlyContinue
        Write-Host "  Removed 'MagicBridge' from $RunKey" -ForegroundColor Green
    } else {
        Write-Host "  No autostart registry entry found." -ForegroundColor Gray
    }
} catch {
    Write-Warning "Could not remove registry autostart entry: $_"
}

# 3. Remove Start Menu shortcuts
Write-Host "`n[3/4] Removing Start Menu shortcuts..." -ForegroundColor Cyan
$StartMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\MagicBridge"
if (Test-Path $StartMenuDir) {
    Remove-Item -Path $StartMenuDir -Recurse -Force
    Write-Host "  Removed Start Menu folder: $StartMenuDir" -ForegroundColor Green
} else {
    Write-Host "  No Start Menu folder found." -ForegroundColor Gray
}

# 4. Remove installation files & optional data purge
Write-Host "`n[4/4] Removing application files..." -ForegroundColor Cyan
if (Test-Path $TargetDir) {
    # If the uninstaller is running from within TargetDir, copy itself to temp to remove TargetDir
    $CurrentScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
    if ($CurrentScriptDir.TrimEnd('\') -eq $TargetDir.TrimEnd('\')) {
        Write-Host "  Uninstaller is running from within the target directory." -ForegroundColor Yellow
        Write-Host "  Cleaning up directory contents..."
        Get-ChildItem -Path $TargetDir -Exclude "uninstall.ps1" | Remove-Item -Recurse -Force
        Write-Host "  Note: The uninstaller script itself will remain until closed." -ForegroundColor Yellow
    } else {
        Remove-Item -Path $TargetDir -Recurse -Force
        Write-Host "  Removed installation directory: $TargetDir" -ForegroundColor Green
    }
}

if ($PurgeData) {
    $AppDataDir = Join-Path $env:APPDATA "MagicBridge"
    if (Test-Path $AppDataDir) {
        Remove-Item -Path $AppDataDir -Recurse -Force
        Write-Host "  Purged user configuration directory: $AppDataDir" -ForegroundColor Green
    }
} else {
    Write-Host "  User settings in %APPDATA%\MagicBridge were preserved." -ForegroundColor Gray
    Write-Host "  (To remove them completely, run with -PurgeData)" -ForegroundColor Gray
}

Write-Host "`n==================================================" -ForegroundColor Green
Write-Host "      MAGICBRIDGE UNINSTALLATION COMPLETED        " -ForegroundColor Green
Write-Host "==================================================" -ForegroundColor Green
Write-Host ""
