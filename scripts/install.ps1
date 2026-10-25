<#
.SYNOPSIS
    Installs MagicBridge for the current user without requiring Administrator privileges.

.DESCRIPTION
    Installs MagicBridge into %LOCALAPPDATA%\Programs\MagicBridge, creates Start Menu
    and Desktop shortcuts, registers the Windows Uninstall entry, optionally configures autostart
    on Windows logon (HKCU), and launches the background agent.

.PARAMETER TargetDir
    Installation directory. Defaults to "$env:LOCALAPPDATA\Programs\MagicBridge".

.PARAMETER CreateDesktopShortcut
    Creates a desktop shortcut to MagicBridge Settings. Default is on.

.PARAMETER NoDesktopShortcut
    Disables creating the desktop shortcut.

.PARAMETER CreateStartMenuShortcut
    Creates Start Menu shortcuts to Settings and Agent. Default is on.

.PARAMETER NoStartMenuShortcut
    Disables creating Start Menu shortcuts.

.PARAMETER EnableAutostart
    Configures MagicBridge to launch automatically on Windows login (per-user HKCU). Default is on.

.PARAMETER NoAutostart
    Disables autostart on Windows login.

.PARAMETER LaunchNow
    Starts the MagicBridge background agent immediately after installation. Default is on.

.PARAMETER NoLaunch
    Prevents launching the background agent immediately after installation.

.EXAMPLE
    .\install.ps1
    .\install.ps1 -TargetDir "C:\Tools\MagicBridge" -NoAutostart
#>

[CmdletBinding()]
param (
    [string]$TargetDir = "$env:LOCALAPPDATA\Programs\MagicBridge",
    [switch]$CreateDesktopShortcut,
    [switch]$NoDesktopShortcut,
    [switch]$CreateStartMenuShortcut,
    [switch]$NoStartMenuShortcut,
    [switch]$EnableAutostart,
    [switch]$NoAutostart,
    [switch]$LaunchNow,
    [switch]$NoLaunch
)

$ErrorActionPreference = "Stop"

$ResolvedDesktopShortcut = if ($NoDesktopShortcut) { $false } else { $true }
$ResolvedStartMenuShortcut = if ($NoStartMenuShortcut) { $false } else { $true }
$ResolvedAutostart = if ($NoAutostart) { $false } else { $true }
$ResolvedLaunchNow = if ($NoLaunch) { $false } else { $true }

Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "         MagicBridge User-Space Installer         " -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan

# Source files location (either repository root or unpacked release archive)
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
if (-not $ScriptDir) { $ScriptDir = (Get-Location).Path }

# Locate agent binary
$CandidateAgentPaths = @(
    Join-Path $ScriptDir "magicbridge-agent.exe",
    Join-Path $ScriptDir "..\target\release\magicbridge-agent.exe",
    Join-Path $ScriptDir "target\release\magicbridge-agent.exe"
)
$SourceAgent = $CandidateAgentPaths | Where-Object { Test-Path $_ } | Select-Object -First 1

if (-not $SourceAgent) {
    throw "magicbridge-agent.exe not found in $ScriptDir or target\release. Please run package-release.ps1 or build the project first."
}
$SourceDir = Split-Path -Parent $SourceAgent

# Resolve version
$Version = "0.2.0"
try {
    $verInfo = (Get-Item $SourceAgent).VersionInfo.ProductVersion
    if ($verInfo) { $Version = $verInfo.Trim() }
} catch {}
if ($Version -eq "0.2.0") {
    $CargoToml = Join-Path $ScriptDir "..\Cargo.toml"
    if (Test-Path $CargoToml) {
        $content = Get-Content $CargoToml -Raw
        if ($content -match '\[workspace\.package\][\s\S]*?version\s*=\s*"([^"]+)"') {
            $Version = $matches[1].Trim()
        }
    }
}

Write-Host "Source directory: $SourceDir"
Write-Host "Target directory: $TargetDir"
Write-Host "Version:          v$Version"

# Stop running instances if any
Write-Host "`nStopping any existing MagicBridge processes..." -ForegroundColor Yellow
Get-Process -Name "magicbridge-agent", "magicbridge-settings" -ErrorAction SilentlyContinue | ForEach-Object {
    Write-Host "  Stopping $($_.ProcessName) (PID $($_.Id))..."
    try {
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    } catch {}
}
Start-Sleep -Milliseconds 400

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
    Get-ChildItem -Path $LocalesSource -Filter "*.toml" | ForEach-Object {
        Copy-Item $_.FullName -Destination $TargetLocales -Force
    }
}

# Copy documentation and scripts
@("README.md", "LICENSE", "user-guide.md") | ForEach-Object {
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
$SetupScript = Join-Path $ScriptDir "setup-wizard.ps1"
if (Test-Path $SetupScript) {
    Copy-Item $SetupScript -Destination $TargetDir -Force
}

$WshShell = New-Object -ComObject WScript.Shell

# Create Start Menu Shortcuts
if ($ResolvedStartMenuShortcut) {
    Write-Host "Creating Start Menu shortcuts..." -ForegroundColor Cyan
    $StartMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\MagicBridge"
    if (-not (Test-Path $StartMenuDir)) {
        New-Item -ItemType Directory -Path $StartMenuDir -Force | Out-Null
    }

    $SettingsShortcut = $WshShell.CreateShortcut((Join-Path $StartMenuDir "MagicBridge Settings.lnk"))
    $SettingsShortcut.TargetPath = Join-Path $TargetDir "magicbridge-settings.exe"
    $SettingsShortcut.WorkingDirectory = $TargetDir
    $SettingsShortcut.Description = "Configure Apple Magic Keyboard mappings and profiles."
    $SettingsShortcut.Save()

    $AgentShortcut = $WshShell.CreateShortcut((Join-Path $StartMenuDir "MagicBridge Agent.lnk"))
    $AgentShortcut.TargetPath = Join-Path $TargetDir "magicbridge-agent.exe"
    $AgentShortcut.WorkingDirectory = $TargetDir
    $AgentShortcut.Description = "Background keyboard remap hook and battery monitor."
    $AgentShortcut.Save()
}

# Create Desktop Shortcut
if ($ResolvedDesktopShortcut) {
    Write-Host "Creating Desktop shortcut..." -ForegroundColor Cyan
    $DesktopDir = [Environment]::GetFolderPath([Environment+SpecialFolder]::Desktop)
    if (Test-Path $DesktopDir) {
        $DesktopShortcut = $WshShell.CreateShortcut((Join-Path $DesktopDir "MagicBridge Settings.lnk"))
        $DesktopShortcut.TargetPath = Join-Path $TargetDir "magicbridge-settings.exe"
        $DesktopShortcut.WorkingDirectory = $TargetDir
        $DesktopShortcut.Description = "MagicBridge - Apple Magic Keyboard for Windows."
        $DesktopShortcut.Save()
    }
}

# Configure autostart in registry
if ($ResolvedAutostart) {
    Write-Host "Configuring autostart in HKCU Run registry..." -ForegroundColor Cyan
    $RunKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
    $AgentBinaryPath = "`"$TargetDir\magicbridge-agent.exe`""
    Set-ItemProperty -Path $RunKey -Name "MagicBridge" -Value $AgentBinaryPath
    Write-Host "  Autostart registered: $AgentBinaryPath" -ForegroundColor Green
}

# Register Windows Add/Remove Programs
Write-Host "Registering in Windows Add/Remove Programs..." -ForegroundColor Cyan
$UninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\MagicBridge"
if (-not (Test-Path $UninstallKey)) {
    New-Item -Path $UninstallKey -Force | Out-Null
}
$TotalBytes = (Get-ChildItem -Path $TargetDir -Recurse -File -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum
$EstimatedKB = [int][math]::Round(($TotalBytes / 1KB))

Set-ItemProperty -Path $UninstallKey -Name "DisplayName" -Value "MagicBridge (Apple Magic Keyboard for Windows)"
Set-ItemProperty -Path $UninstallKey -Name "DisplayVersion" -Value $Version
Set-ItemProperty -Path $UninstallKey -Name "Publisher" -Value "MagicBridge Contributors"
Set-ItemProperty -Path $UninstallKey -Name "InstallLocation" -Value $TargetDir
Set-ItemProperty -Path $UninstallKey -Name "DisplayIcon" -Value "$TargetDir\magicbridge-agent.exe,0"
Set-ItemProperty -Path $UninstallKey -Name "UninstallString" -Value "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$TargetDir\uninstall.ps1`""
Set-ItemProperty -Path $UninstallKey -Name "QuietUninstallString" -Value "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$TargetDir\uninstall.ps1`" -Silent"
Set-ItemProperty -Path $UninstallKey -Name "InstallDate" -Value ((Get-Date).ToString("yyyyMMdd"))
Set-ItemProperty -Path $UninstallKey -Name "EstimatedSize" -Value $EstimatedKB -Type DWord
Set-ItemProperty -Path $UninstallKey -Name "URLInfoAbout" -Value "https://github.com/DooperDrew/magicbridge"
Set-ItemProperty -Path $UninstallKey -Name "HelpLink" -Value "https://github.com/DooperDrew/magicbridge/issues"
Set-ItemProperty -Path $UninstallKey -Name "URLUpdateInfo" -Value "https://github.com/DooperDrew/magicbridge/releases"
Set-ItemProperty -Path $UninstallKey -Name "NoModify" -Value 1 -Type DWord
Set-ItemProperty -Path $UninstallKey -Name "NoRepair" -Value 1 -Type DWord

# Launch agent if requested
if ($ResolvedLaunchNow) {
    Write-Host "Launching MagicBridge background agent..." -ForegroundColor Cyan
    Start-Process -FilePath (Join-Path $TargetDir "magicbridge-agent.exe") -WorkingDirectory $TargetDir
}

Write-Host "`n==================================================" -ForegroundColor Green
Write-Host "       MAGICBRIDGE INSTALLATION COMPLETED         " -ForegroundColor Green
Write-Host "==================================================" -ForegroundColor Green
Write-Host "Installation path: $TargetDir"
Write-Host "Configuration dir: $env:APPDATA\MagicBridge"
Write-Host ""
