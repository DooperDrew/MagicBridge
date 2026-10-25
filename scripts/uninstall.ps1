<#
.SYNOPSIS
    Uninstalls MagicBridge, removes autostart entries, shortcuts, and application files.

.DESCRIPTION
    Stops running MagicBridge processes, removes the autostart entry from the
    current user registry (HKCU\Software\Microsoft\Windows\CurrentVersion\Run),
    removes the Windows Add/Remove Programs registration (HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\MagicBridge),
    deletes Start Menu and Desktop shortcuts, removes program files, and optionally purges configuration data
    in %APPDATA%\MagicBridge.

.PARAMETER TargetDir
    The installation directory to remove. Defaults to the folder containing this script if installed,
    or "$env:LOCALAPPDATA\Programs\MagicBridge".

.PARAMETER PurgeData
    If specified, permanently removes user settings and logs in %APPDATA%\MagicBridge without prompting.

.PARAMETER Silent
    Runs the uninstaller silently without prompts or message boxes.

.EXAMPLE
    .\uninstall.ps1
    .\uninstall.ps1 -Silent
    .\uninstall.ps1 -PurgeData
#>

[CmdletBinding()]
param (
    [string]$TargetDir,
    [switch]$PurgeData,
    [switch]$Silent
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
if (-not $ScriptDir) {
    $ScriptDir = (Get-Location).Path
}

# Auto-detect TargetDir if not explicitly supplied
if (-not $TargetDir) {
    if (Test-Path (Join-Path $ScriptDir "magicbridge-agent.exe")) {
        $TargetDir = $ScriptDir
    } else {
        $TargetDir = "$env:LOCALAPPDATA\Programs\MagicBridge"
    }
}

# If not running in silent mode and interactive GUI is possible, confirm with user
if (-not $Silent) {
    try {
        Add-Type -AssemblyName System.Windows.Forms -ErrorAction SilentlyContinue
        $confirm = [System.Windows.Forms.MessageBox]::Show(
            "Are you sure you want to completely uninstall MagicBridge (Apple Magic Keyboard for Windows)?",
            "Uninstall MagicBridge",
            [System.Windows.Forms.MessageBoxButtons]::YesNo,
            [System.Windows.Forms.MessageBoxIcon]::Question
        )
        if ($confirm -ne [System.Windows.Forms.DialogResult]::Yes) {
            Write-Host "Uninstallation cancelled by user."
            return
        }

        if (-not $PurgeData) {
            $askPurge = [System.Windows.Forms.MessageBox]::Show(
                "Do you also want to remove your custom keyboard profiles and settings stored in %APPDATA%\MagicBridge?",
                "Remove User Data?",
                [System.Windows.Forms.MessageBoxButtons]::YesNo,
                [System.Windows.Forms.MessageBoxIcon]::Question
            )
            if ($askPurge -eq [System.Windows.Forms.DialogResult]::Yes) {
                $PurgeData = $true
            }
        }
    } catch {
        # Fallback to console prompt if WinForms dialog is unavailable
        Write-Host "==================================================" -ForegroundColor Cyan
        Write-Host "         MagicBridge Uninstallation Tool          " -ForegroundColor Cyan
        Write-Host "==================================================" -ForegroundColor Cyan
        $ans = Read-Host "Are you sure you want to uninstall MagicBridge? (y/N)"
        if ($ans -notmatch "^[yY]") {
            Write-Host "Uninstallation cancelled."
            return
        }
    }
}

Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "         MagicBridge Uninstallation Tool          " -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "Target Directory: $TargetDir"

# 1. Stop running processes
Write-Host "`n[1/5] Stopping running MagicBridge processes..." -ForegroundColor Cyan
Get-Process -Name "magicbridge-agent", "magicbridge-settings" -ErrorAction SilentlyContinue | ForEach-Object {
    Write-Host "  Stopping $($_.ProcessName) (PID $($_.Id))..."
    try {
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    } catch {}
}
Start-Sleep -Milliseconds 400

# 2. Remove autostart entry from registry
Write-Host "`n[2/5] Removing autostart entry from HKCU Run..." -ForegroundColor Cyan
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

# 3. Remove Windows Add/Remove Programs (Uninstall) registry entry
Write-Host "`n[3/5] Removing Windows Add/Remove Programs entry..." -ForegroundColor Cyan
$UninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\MagicBridge"
try {
    if (Test-Path $UninstallKey) {
        Remove-Item -Path $UninstallKey -Recurse -Force -ErrorAction SilentlyContinue
        Write-Host "  Removed registry key: $UninstallKey" -ForegroundColor Green
    } else {
        Write-Host "  No Uninstall registry key found." -ForegroundColor Gray
    }
} catch {
    Write-Warning "Could not remove registry uninstall entry: $_"
}

# 4. Remove Shortcuts (Start Menu & Desktop)
Write-Host "`n[4/5] Removing shortcuts..." -ForegroundColor Cyan
$StartMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\MagicBridge"
if (Test-Path $StartMenuDir) {
    Remove-Item -Path $StartMenuDir -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "  Removed Start Menu folder: $StartMenuDir" -ForegroundColor Green
} else {
    Write-Host "  No Start Menu folder found." -ForegroundColor Gray
}

$DesktopDir = [Environment]::GetFolderPath([Environment+SpecialFolder]::Desktop)
if (Test-Path $DesktopDir) {
    @("MagicBridge Settings.lnk", "MagicBridge Agent.lnk", "MagicBridge.lnk") | ForEach-Object {
        $deskLnk = Join-Path $DesktopDir $_
        if (Test-Path $deskLnk) {
            Remove-Item -Path $deskLnk -Force -ErrorAction SilentlyContinue
            Write-Host "  Removed Desktop shortcut: $deskLnk" -ForegroundColor Green
        }
    }
}

# 5. Remove installation files & optional data purge
Write-Host "`n[5/5] Removing application files..." -ForegroundColor Cyan
if (Test-Path $TargetDir) {
    if ($ScriptDir -and ($ScriptDir.TrimEnd('\') -eq $TargetDir.TrimEnd('\'))) {
        Write-Host "  Uninstaller is running from within target directory: $TargetDir" -ForegroundColor Yellow
        # Delete everything except uninstall.ps1 first
        Get-ChildItem -Path $TargetDir -Exclude "uninstall.ps1" | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
        # Self-delete directory via background cmd process once powershell exits
        Start-Process cmd.exe -ArgumentList "/c cd /d %TEMP% & timeout /t 1 /nobreak >nul & rmdir /s /q `"$TargetDir`"" -WindowStyle Hidden
        Write-Host "  Scheduled target directory cleanup on exit." -ForegroundColor Green
    } else {
        Remove-Item -Path $TargetDir -Recurse -Force -ErrorAction SilentlyContinue
        Write-Host "  Removed installation directory: $TargetDir" -ForegroundColor Green
    }
}

if ($PurgeData) {
    $AppDataDir = Join-Path $env:APPDATA "MagicBridge"
    if (Test-Path $AppDataDir) {
        Remove-Item -Path $AppDataDir -Recurse -Force -ErrorAction SilentlyContinue
        Write-Host "  Purged user configuration directory: $AppDataDir" -ForegroundColor Green
    }
} else {
    Write-Host "  User settings in %APPDATA%\MagicBridge were preserved." -ForegroundColor Gray
}

Write-Host "`n==================================================" -ForegroundColor Green
Write-Host "      MAGICBRIDGE UNINSTALLATION COMPLETED        " -ForegroundColor Green
Write-Host "==================================================" -ForegroundColor Green
Write-Host ""

if (-not $Silent) {
    try {
        [System.Windows.Forms.MessageBox]::Show(
            "MagicBridge has been successfully removed from your computer.",
            "Uninstall MagicBridge",
            [System.Windows.Forms.MessageBoxButtons]::OK,
            [System.Windows.Forms.MessageBoxIcon]::Information
        ) | Out-Null
    } catch {}
}
