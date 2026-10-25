<#
.SYNOPSIS
    Guided Windows Setup Wizard for MagicBridge.

.DESCRIPTION
    Provides a zero-elevation graphical (Windows Forms) and interactive wizard to install
    MagicBridge into %LOCALAPPDATA%\Programs\MagicBridge, configure Start Menu and Desktop
    shortcuts, set up logon autostart, register Windows Add/Remove Programs entry,
    and launch initial setup / settings.

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
    Registers MagicBridge to launch automatically on Windows login (HKCU). Default is on.

.PARAMETER NoAutostart
    Disables autostart on Windows login.

.PARAMETER LaunchSettingsOnFinish
    Launches MagicBridge Settings upon installation completion. Default is on.

.PARAMETER NoLaunch
    Prevents launching MagicBridge Settings/Agent upon completion.

.PARAMETER Silent
    Runs the installation unattended with no GUI or prompts using provided parameters.

.PARAMETER NoGui
    Forces interactive console wizard instead of Windows Forms GUI.

.EXAMPLE
    .\setup-wizard.ps1
    .\setup-wizard.ps1 -Silent -NoLaunch
    .\setup-wizard.ps1 -NoGui
    .\setup-wizard.ps1 -TargetDir "C:\Custom\MagicBridge" -NoAutostart
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
    [switch]$LaunchSettingsOnFinish,
    [switch]$NoLaunch,
    [switch]$Silent,
    [switch]$NoGui
)

$ErrorActionPreference = "Stop"

$ResolvedDesktopShortcut = if ($NoDesktopShortcut) { $false } else { $true }
$ResolvedStartMenuShortcut = if ($NoStartMenuShortcut) { $false } else { $true }
$ResolvedAutostart = if ($NoAutostart) { $false } else { $true }
$ResolvedLaunchOnFinish = if ($NoLaunch) { $false } else { $true }

# -----------------------------------------------------------------------------
# 1. Locate Source Binaries & Assets
# -----------------------------------------------------------------------------
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
if (-not $ScriptDir) {
    $ScriptDir = (Get-Location).Path
}

$CandidateSources = @(
    $ScriptDir,
    (Join-Path $ScriptDir "..\target\release"),
    (Join-Path $ScriptDir "target\release")
)

$SourceDir = $null
foreach ($cand in $CandidateSources) {
    if (Test-Path (Join-Path $cand "magicbridge-agent.exe")) {
        $SourceDir = (Resolve-Path $cand).Path
        break
    }
}

if (-not $SourceDir) {
    throw "magicbridge-agent.exe was not found in $ScriptDir or target\release. Please build the workspace or run from an extracted release package."
}

# Resolve version
$Version = "0.2.0"
$AgentExePath = Join-Path $SourceDir "magicbridge-agent.exe"
if (Test-Path $AgentExePath) {
    try {
        $verInfo = (Get-Item $AgentExePath).VersionInfo.ProductVersion
        if ($verInfo) { $Version = $verInfo.Trim() }
    } catch {}
}
if ($Version -eq "0.2.0") {
    $CargoToml = Join-Path $ScriptDir "..\Cargo.toml"
    if (Test-Path $CargoToml) {
        $content = Get-Content $CargoToml -Raw
        if ($content -match '\[workspace\.package\][\s\S]*?version\s*=\s*"([^"]+)"') {
            $Version = $matches[1].Trim()
        }
    }
}

# -----------------------------------------------------------------------------
# 2. Core Installation Worker Function
# -----------------------------------------------------------------------------
function Install-MagicBridgePackage {
    param (
        [string]$DestinationDir,
        [bool]$AddDesktopShortcut,
        [bool]$AddStartMenuShortcut,
        [bool]$SetAutostart,
        [scriptblock]$ProgressCallback
    )

    function Report-Status([string]$msg, [int]$percent) {
        if ($ProgressCallback) {
            & $ProgressCallback $msg $percent
        } else {
            Write-Host "[$percent%] $msg"
        }
    }

    Report-Status "Stopping any running MagicBridge processes..." 10
    Get-Process -Name "magicbridge-agent", "magicbridge-settings" -ErrorAction SilentlyContinue | ForEach-Object {
        try {
            Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
        } catch {}
    }
    Start-Sleep -Milliseconds 400

    Report-Status "Creating installation directory: $DestinationDir" 25
    if (-not (Test-Path $DestinationDir)) {
        New-Item -ItemType Directory -Path $DestinationDir -Force | Out-Null
    }

    $TargetLocales = Join-Path $DestinationDir "locales"
    if (-not (Test-Path $TargetLocales)) {
        New-Item -ItemType Directory -Path $TargetLocales -Force | Out-Null
    }

    Report-Status "Copying application binaries and components..." 45
    Copy-Item (Join-Path $SourceDir "magicbridge-agent.exe") -Destination $DestinationDir -Force
    $SettingsSource = Join-Path $SourceDir "magicbridge-settings.exe"
    if (Test-Path $SettingsSource) {
        Copy-Item $SettingsSource -Destination $DestinationDir -Force
    }

    # Locales
    $LocalesSource = Join-Path $SourceDir "locales"
    if (-not (Test-Path $LocalesSource)) {
        $LocalesSource = Join-Path $ScriptDir "..\crates\settings\locales"
    }
    if (Test-Path $LocalesSource) {
        Get-ChildItem -Path $LocalesSource -Filter "*.toml" | ForEach-Object {
            Copy-Item $_.FullName -Destination $TargetLocales -Force
        }
    }

    # Documentation and scripts
    @("README.md", "LICENSE", "user-guide.md") | ForEach-Object {
        $doc = Join-Path $SourceDir $_
        if (-not (Test-Path $doc)) {
            $doc = Join-Path $ScriptDir "..\\$_"
        }
        if (Test-Path $doc) {
            Copy-Item $doc -Destination $DestinationDir -Force
        }
    }

    $UninstallScript = Join-Path $ScriptDir "uninstall.ps1"
    if (Test-Path $UninstallScript) {
        Copy-Item $UninstallScript -Destination $DestinationDir -Force
    }
    $SetupScript = Join-Path $ScriptDir "setup-wizard.ps1"
    if (Test-Path $SetupScript) {
        Copy-Item $SetupScript -Destination $DestinationDir -Force
    }

    Report-Status "Configuring Windows shortcuts..." 70
    $WshShell = New-Object -ComObject WScript.Shell

    if ($AddStartMenuShortcut) {
        $StartMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\MagicBridge"
        if (-not (Test-Path $StartMenuDir)) {
            New-Item -ItemType Directory -Path $StartMenuDir -Force | Out-Null
        }

        # Settings shortcut
        $s1 = $WshShell.CreateShortcut((Join-Path $StartMenuDir "MagicBridge Settings.lnk"))
        $s1.TargetPath = Join-Path $DestinationDir "magicbridge-settings.exe"
        $s1.WorkingDirectory = $DestinationDir
        $s1.Description = "Configure Apple Magic Keyboard mappings and profiles."
        $s1.Save()

        # Agent shortcut
        $s2 = $WshShell.CreateShortcut((Join-Path $StartMenuDir "MagicBridge Agent.lnk"))
        $s2.TargetPath = Join-Path $DestinationDir "magicbridge-agent.exe"
        $s2.WorkingDirectory = $DestinationDir
        $s2.Description = "Background keyboard remap hook and battery monitor."
        $s2.Save()
    }

    if ($AddDesktopShortcut) {
        $DesktopDir = [Environment]::GetFolderPath([Environment+SpecialFolder]::Desktop)
        if (Test-Path $DesktopDir) {
            $sDesk = $WshShell.CreateShortcut((Join-Path $DesktopDir "MagicBridge Settings.lnk"))
            $sDesk.TargetPath = Join-Path $DestinationDir "magicbridge-settings.exe"
            $sDesk.WorkingDirectory = $DestinationDir
            $sDesk.Description = "MagicBridge - Apple Magic Keyboard for Windows."
            $sDesk.Save()
        }
    }

    Report-Status "Configuring autostart at logon..." 85
    $RunKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
    if ($SetAutostart) {
        $AgentBinaryPath = "`"$DestinationDir\magicbridge-agent.exe`""
        Set-ItemProperty -Path $RunKey -Name "MagicBridge" -Value $AgentBinaryPath
    } else {
        try {
            Remove-ItemProperty -Path $RunKey -Name "MagicBridge" -ErrorAction SilentlyContinue
        } catch {}
    }

    Report-Status "Registering Windows Add/Remove Programs entry..." 95
    $UninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\MagicBridge"
    if (-not (Test-Path $UninstallKey)) {
        New-Item -Path $UninstallKey -Force | Out-Null
    }

    $TotalBytes = (Get-ChildItem -Path $DestinationDir -Recurse -File -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum
    $EstimatedKB = [int][math]::Round(($TotalBytes / 1KB))

    Set-ItemProperty -Path $UninstallKey -Name "DisplayName" -Value "MagicBridge (Apple Magic Keyboard for Windows)"
    Set-ItemProperty -Path $UninstallKey -Name "DisplayVersion" -Value $Version
    Set-ItemProperty -Path $UninstallKey -Name "Publisher" -Value "MagicBridge Contributors"
    Set-ItemProperty -Path $UninstallKey -Name "InstallLocation" -Value $DestinationDir
    Set-ItemProperty -Path $UninstallKey -Name "DisplayIcon" -Value "$DestinationDir\magicbridge-agent.exe,0"
    Set-ItemProperty -Path $UninstallKey -Name "UninstallString" -Value "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$DestinationDir\uninstall.ps1`""
    Set-ItemProperty -Path $UninstallKey -Name "QuietUninstallString" -Value "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$DestinationDir\uninstall.ps1`" -Silent"
    Set-ItemProperty -Path $UninstallKey -Name "InstallDate" -Value ((Get-Date).ToString("yyyyMMdd"))
    Set-ItemProperty -Path $UninstallKey -Name "EstimatedSize" -Value $EstimatedKB -Type DWord
    Set-ItemProperty -Path $UninstallKey -Name "URLInfoAbout" -Value "https://github.com/DooperDrew/test-repo-antigravity"
    Set-ItemProperty -Path $UninstallKey -Name "HelpLink" -Value "https://github.com/DooperDrew/test-repo-antigravity/issues"
    Set-ItemProperty -Path $UninstallKey -Name "URLUpdateInfo" -Value "https://github.com/DooperDrew/test-repo-antigravity/releases"
    Set-ItemProperty -Path $UninstallKey -Name "NoModify" -Value 1 -Type DWord
    Set-ItemProperty -Path $UninstallKey -Name "NoRepair" -Value 1 -Type DWord

    Report-Status "MagicBridge installation complete!" 100
}

# -----------------------------------------------------------------------------
# 3. Execution: Silent Mode
# -----------------------------------------------------------------------------
if ($Silent) {
    Write-Host "MagicBridge Silent Setup starting..." -ForegroundColor Cyan
    Write-Host "Target Directory: $TargetDir"
    Install-MagicBridgePackage `
        -DestinationDir $TargetDir `
        -AddDesktopShortcut:$ResolvedDesktopShortcut `
        -AddStartMenuShortcut:$ResolvedStartMenuShortcut `
        -SetAutostart:$ResolvedAutostart `
        -ProgressCallback {
            param($msg, $pct)
            Write-Host "[$pct%] $msg"
        }

    if ($ResolvedLaunchOnFinish) {
        Write-Host "Launching MagicBridge background agent..." -ForegroundColor Cyan
        Start-Process -FilePath (Join-Path $TargetDir "magicbridge-agent.exe") -WorkingDirectory $TargetDir
        $SettingsExe = Join-Path $TargetDir "magicbridge-settings.exe"
        if (Test-Path $SettingsExe) {
            Write-Host "Launching MagicBridge settings..." -ForegroundColor Cyan
            Start-Process -FilePath $SettingsExe -WorkingDirectory $TargetDir
        }
    }
    Write-Host "Silent installation completed successfully." -ForegroundColor Green
    return
}

# -----------------------------------------------------------------------------
# 4. Execution: Console Mode (-NoGui)
# -----------------------------------------------------------------------------
if ($NoGui) {
    Write-Host "==================================================" -ForegroundColor Cyan
    Write-Host "         MagicBridge Guided Setup (Console)       " -ForegroundColor Cyan
    Write-Host "==================================================" -ForegroundColor Cyan
    Write-Host "Version: v$Version"
    Write-Host "Source:  $SourceDir`n"

    $inputDir = Read-Host "Installation Directory [$TargetDir]"
    if (-not [string]::IsNullOrWhiteSpace($inputDir)) {
        $TargetDir = $inputDir
    }

    $ansStartMenu = Read-Host "Create Start Menu shortcuts? (Y/n)"
    $ResolvedStartMenuShortcut = ($ansStartMenu -notmatch "^[nN]")

    $ansDesktop = Read-Host "Create Desktop shortcut for Settings? (Y/n)"
    $ResolvedDesktopShortcut = ($ansDesktop -notmatch "^[nN]")

    $ansAutostart = Read-Host "Start MagicBridge automatically on Windows logon? (Y/n)"
    $ResolvedAutostart = ($ansAutostart -notmatch "^[nN]")

    $ansLaunch = Read-Host "Launch MagicBridge Settings immediately upon completion? (Y/n)"
    $ResolvedLaunchOnFinish = ($ansLaunch -notmatch "^[nN]")

    Write-Host "`nStarting installation..." -ForegroundColor Cyan
    Install-MagicBridgePackage `
        -DestinationDir $TargetDir `
        -AddDesktopShortcut:$ResolvedDesktopShortcut `
        -AddStartMenuShortcut:$ResolvedStartMenuShortcut `
        -SetAutostart:$ResolvedAutostart `
        -ProgressCallback {
            param($msg, $pct)
            Write-Host "[$pct%] $msg" -ForegroundColor Gray
        }

    if ($ResolvedLaunchOnFinish) {
        Start-Process -FilePath (Join-Path $TargetDir "magicbridge-agent.exe") -WorkingDirectory $TargetDir
        $SettingsExe = Join-Path $TargetDir "magicbridge-settings.exe"
        if (Test-Path $SettingsExe) {
            Start-Process -FilePath $SettingsExe -WorkingDirectory $TargetDir
        }
    }

    Write-Host "`nInstallation completed successfully." -ForegroundColor Green
    Write-Host "MagicBridge is now ready to use at: $TargetDir`n"
    return
}

# -----------------------------------------------------------------------------
# 5. Execution: Graphical Windows Forms Setup Wizard
# -----------------------------------------------------------------------------
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$form = New-Object System.Windows.Forms.Form
$form.Text = "MagicBridge Setup Wizard"
$form.ClientSize = New-Object System.Drawing.Size(580, 440)
$form.StartPosition = [System.Windows.Forms.FormStartPosition]::CenterScreen
$form.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::FixedSingle
$form.MaximizeBox = $false
$form.MinimizeBox = $true
$form.BackColor = [System.Drawing.Color]::White
$form.Font = New-Object System.Drawing.Font("Segoe UI", 9)

# ----------------- Top Header Panel -----------------
$pnlHeader = New-Object System.Windows.Forms.Panel
$pnlHeader.Location = New-Object System.Drawing.Point(0, 0)
$pnlHeader.Size = New-Object System.Drawing.Size(580, 70)
$pnlHeader.BackColor = [System.Drawing.Color]::FromArgb(26, 30, 38)
$form.Controls.Add($pnlHeader)

$lblTitle = New-Object System.Windows.Forms.Label
$lblTitle.Text = "MagicBridge Setup"
$lblTitle.Font = New-Object System.Drawing.Font("Segoe UI", 14, [System.Drawing.FontStyle]::Bold)
$lblTitle.ForeColor = [System.Drawing.Color]::White
$lblTitle.Location = New-Object System.Drawing.Point(22, 12)
$lblTitle.AutoSize = $true
$pnlHeader.Controls.Add($lblTitle)

$lblSubtitle = New-Object System.Windows.Forms.Label
$lblSubtitle.Text = "Apple Magic Keyboard with Numeric Keypad (A1843) for Windows  •  v$Version"
$lblSubtitle.Font = New-Object System.Drawing.Font("Segoe UI", 8.5)
$lblSubtitle.ForeColor = [System.Drawing.Color]::FromArgb(180, 195, 215)
$lblSubtitle.Location = New-Object System.Drawing.Point(24, 40)
$lblSubtitle.AutoSize = $true
$pnlHeader.Controls.Add($lblSubtitle)

$pnlAccent = New-Object System.Windows.Forms.Panel
$pnlAccent.Location = New-Object System.Drawing.Point(0, 68)
$pnlAccent.Size = New-Object System.Drawing.Size(580, 2)
$pnlAccent.BackColor = [System.Drawing.Color]::FromArgb(0, 120, 212)
$pnlHeader.Controls.Add($pnlAccent)

# ----------------- Bottom Footer Panel -----------------
$pnlFooter = New-Object System.Windows.Forms.Panel
$pnlFooter.Location = New-Object System.Drawing.Point(0, 385)
$pnlFooter.Size = New-Object System.Drawing.Size(580, 55)
$pnlFooter.BackColor = [System.Drawing.Color]::FromArgb(243, 243, 243)
$form.Controls.Add($pnlFooter)

$pnlFooterLine = New-Object System.Windows.Forms.Panel
$pnlFooterLine.Location = New-Object System.Drawing.Point(0, 0)
$pnlFooterLine.Size = New-Object System.Drawing.Size(580, 1)
$pnlFooterLine.BackColor = [System.Drawing.Color]::FromArgb(218, 218, 218)
$pnlFooter.Controls.Add($pnlFooterLine)

$btnBack = New-Object System.Windows.Forms.Button
$btnBack.Text = "< Back"
$btnBack.Location = New-Object System.Drawing.Point(305, 14)
$btnBack.Size = New-Object System.Drawing.Size(75, 28)
$btnBack.Enabled = $false
$pnlFooter.Controls.Add($btnBack)

$btnNext = New-Object System.Windows.Forms.Button
$btnNext.Text = "Next >"
$btnNext.Location = New-Object System.Drawing.Point(390, 14)
$btnNext.Size = New-Object System.Drawing.Size(85, 28)
$btnNext.BackColor = [System.Drawing.Color]::FromArgb(0, 120, 212)
$btnNext.ForeColor = [System.Drawing.Color]::White
$btnNext.FlatStyle = [System.Windows.Forms.FlatStyle]::Flat
$btnNext.FlatAppearance.BorderSize = 0
$pnlFooter.Controls.Add($btnNext)

$btnCancel = New-Object System.Windows.Forms.Button
$btnCancel.Text = "Cancel"
$btnCancel.Location = New-Object System.Drawing.Point(485, 14)
$btnCancel.Size = New-Object System.Drawing.Size(75, 28)
$pnlFooter.Controls.Add($btnCancel)

# ----------------- Step 1: Welcome Panel -----------------
$pnlWelcome = New-Object System.Windows.Forms.Panel
$pnlWelcome.Location = New-Object System.Drawing.Point(0, 70)
$pnlWelcome.Size = New-Object System.Drawing.Size(580, 315)
$pnlWelcome.BackColor = [System.Drawing.Color]::White
$form.Controls.Add($pnlWelcome)

$lblWelcomeHeading = New-Object System.Windows.Forms.Label
$lblWelcomeHeading.Text = "Welcome to the MagicBridge Setup Wizard"
$lblWelcomeHeading.Font = New-Object System.Drawing.Font("Segoe UI", 12, [System.Drawing.FontStyle]::Bold)
$lblWelcomeHeading.ForeColor = [System.Drawing.Color]::FromArgb(30, 30, 30)
$lblWelcomeHeading.Location = New-Object System.Drawing.Point(25, 20)
$lblWelcomeHeading.AutoSize = $true
$pnlWelcome.Controls.Add($lblWelcomeHeading)

$lblWelcomeBody = New-Object System.Windows.Forms.Label
$lblWelcomeBody.Text = @"
This wizard will guide you through setting up MagicBridge on your computer.

MagicBridge seamlessly unlocks first-class Windows 10 & 11 integration for the Apple Magic Keyboard with Numeric Keypad (Model A1843):

  • Configurable Fn Layer: Media controls, navigation combos, and F-keys switching
  • Modifier Presets: Switch between Mac (Cmd=Win, Opt=Alt) and Windows layouts
  • Missing System Keys: F13→Print Screen, F14→Scroll Lock, F15→Pause
  • Direct Hardware Battery Monitoring: USB and Bluetooth Classic support
  • Anti-Cheat Safe & Driverless: Zero kernel drivers, 0% idle CPU, <0.1ms latency
  • Zero Elevation Required: Installs cleanly for your user account

Click Next to select your installation preferences.
"@
$lblWelcomeBody.Font = New-Object System.Drawing.Font("Segoe UI", 9)
$lblWelcomeBody.ForeColor = [System.Drawing.Color]::FromArgb(60, 60, 60)
$lblWelcomeBody.Location = New-Object System.Drawing.Point(25, 55)
$lblWelcomeBody.Size = New-Object System.Drawing.Size(530, 240)
$pnlWelcome.Controls.Add($lblWelcomeBody)

# ----------------- Step 2: Options Panel -----------------
$pnlOptions = New-Object System.Windows.Forms.Panel
$pnlOptions.Location = New-Object System.Drawing.Point(0, 70)
$pnlOptions.Size = New-Object System.Drawing.Size(580, 315)
$pnlOptions.BackColor = [System.Drawing.Color]::White
$pnlOptions.Visible = $false
$form.Controls.Add($pnlOptions)

# Destination Group
$grpDest = New-Object System.Windows.Forms.GroupBox
$grpDest.Text = "Installation Location"
$grpDest.Location = New-Object System.Drawing.Point(25, 12)
$grpDest.Size = New-Object System.Drawing.Size(530, 80)
$pnlOptions.Controls.Add($grpDest)

$lblDestPrompt = New-Object System.Windows.Forms.Label
$lblDestPrompt.Text = "Install MagicBridge into the following directory:"
$lblDestPrompt.Location = New-Object System.Drawing.Point(15, 20)
$lblDestPrompt.AutoSize = $true
$grpDest.Controls.Add($lblDestPrompt)

$txtTargetDir = New-Object System.Windows.Forms.TextBox
$txtTargetDir.Text = $TargetDir
$txtTargetDir.Location = New-Object System.Drawing.Point(15, 42)
$txtTargetDir.Size = New-Object System.Drawing.Size(410, 24)
$grpDest.Controls.Add($txtTargetDir)

$btnBrowse = New-Object System.Windows.Forms.Button
$btnBrowse.Text = "Browse..."
$btnBrowse.Location = New-Object System.Drawing.Point(433, 40)
$btnBrowse.Size = New-Object System.Drawing.Size(82, 26)
$btnBrowse.Add_Click({
    $dlg = New-Object System.Windows.Forms.FolderBrowserDialog
    $dlg.Description = "Select MagicBridge Installation Folder"
    $dlg.SelectedPath = $txtTargetDir.Text
    if ($dlg.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
        $txtTargetDir.Text = $dlg.SelectedPath
    }
})
$grpDest.Controls.Add($btnBrowse)

# Options Group
$grpOpts = New-Object System.Windows.Forms.GroupBox
$grpOpts.Text = "Shortcuts & Preferences"
$grpOpts.Location = New-Object System.Drawing.Point(25, 100)
$grpOpts.Size = New-Object System.Drawing.Size(530, 200)
$pnlOptions.Controls.Add($grpOpts)

$chkStartMenu = New-Object System.Windows.Forms.CheckBox
$chkStartMenu.Text = "Create Start Menu shortcuts (MagicBridge Settings & Agent)"
$chkStartMenu.Location = New-Object System.Drawing.Point(15, 25)
$chkStartMenu.Size = New-Object System.Drawing.Size(500, 24)
$chkStartMenu.Checked = $ResolvedStartMenuShortcut
$grpOpts.Controls.Add($chkStartMenu)

$chkDesktop = New-Object System.Windows.Forms.CheckBox
$chkDesktop.Text = "Create Desktop shortcut for MagicBridge Settings"
$chkDesktop.Location = New-Object System.Drawing.Point(15, 55)
$chkDesktop.Size = New-Object System.Drawing.Size(500, 24)
$chkDesktop.Checked = $ResolvedDesktopShortcut
$grpOpts.Controls.Add($chkDesktop)

$chkAutostart = New-Object System.Windows.Forms.CheckBox
$chkAutostart.Text = "Start MagicBridge automatically at Windows sign-in (recommended)"
$chkAutostart.Location = New-Object System.Drawing.Point(15, 85)
$chkAutostart.Size = New-Object System.Drawing.Size(500, 24)
$chkAutostart.Checked = $ResolvedAutostart
$grpOpts.Controls.Add($chkAutostart)

$chkLaunchWizard = New-Object System.Windows.Forms.CheckBox
$chkLaunchWizard.Text = "Launch Initial Setup Wizard / Settings on Finish"
$chkLaunchWizard.Location = New-Object System.Drawing.Point(15, 115)
$chkLaunchWizard.Size = New-Object System.Drawing.Size(500, 24)
$chkLaunchWizard.Checked = $ResolvedLaunchOnFinish
$grpOpts.Controls.Add($chkLaunchWizard)

$lblNote = New-Object System.Windows.Forms.Label
$lblNote.Text = "Note: MagicBridge installs cleanly without UAC prompts or Administrator elevation.`nSettings and custom keymaps are preserved in %APPDATA%\MagicBridge."
$lblNote.Font = New-Object System.Drawing.Font("Segoe UI", 8.25)
$lblNote.ForeColor = [System.Drawing.Color]::Gray
$lblNote.Location = New-Object System.Drawing.Point(15, 152)
$lblNote.Size = New-Object System.Drawing.Size(500, 36)
$grpOpts.Controls.Add($lblNote)

# ----------------- Step 3: Progress Panel -----------------
$pnlProgress = New-Object System.Windows.Forms.Panel
$pnlProgress.Location = New-Object System.Drawing.Point(0, 70)
$pnlProgress.Size = New-Object System.Drawing.Size(580, 315)
$pnlProgress.BackColor = [System.Drawing.Color]::White
$pnlProgress.Visible = $false
$form.Controls.Add($pnlProgress)

$lblProgTitle = New-Object System.Windows.Forms.Label
$lblProgTitle.Text = "Installing MagicBridge..."
$lblProgTitle.Font = New-Object System.Drawing.Font("Segoe UI", 12, [System.Drawing.FontStyle]::Bold)
$lblProgTitle.Location = New-Object System.Drawing.Point(25, 18)
$lblProgTitle.AutoSize = $true
$pnlProgress.Controls.Add($lblProgTitle)

$lblProgStatus = New-Object System.Windows.Forms.Label
$lblProgStatus.Text = "Preparing installation..."
$lblProgStatus.Font = New-Object System.Drawing.Font("Segoe UI", 9)
$lblProgStatus.ForeColor = [System.Drawing.Color]::FromArgb(60, 60, 60)
$lblProgStatus.Location = New-Object System.Drawing.Point(25, 48)
$lblProgStatus.Size = New-Object System.Drawing.Size(530, 20)
$pnlProgress.Controls.Add($lblProgStatus)

$progBar = New-Object System.Windows.Forms.ProgressBar
$progBar.Location = New-Object System.Drawing.Point(25, 75)
$progBar.Size = New-Object System.Drawing.Size(530, 20)
$progBar.Style = [System.Windows.Forms.ProgressBarStyle]::Continuous
$progBar.Value = 0
$pnlProgress.Controls.Add($progBar)

$lstLog = New-Object System.Windows.Forms.ListBox
$lstLog.Location = New-Object System.Drawing.Point(25, 105)
$lstLog.Size = New-Object System.Drawing.Size(530, 190)
$lstLog.Font = New-Object System.Drawing.Font("Consolas", 8.5)
$lstLog.BackColor = [System.Drawing.Color]::FromArgb(250, 250, 250)
$pnlProgress.Controls.Add($lstLog)

# ----------------- Step 4: Finished Panel -----------------
$pnlFinished = New-Object System.Windows.Forms.Panel
$pnlFinished.Location = New-Object System.Drawing.Point(0, 70)
$pnlFinished.Size = New-Object System.Drawing.Size(580, 315)
$pnlFinished.BackColor = [System.Drawing.Color]::White
$pnlFinished.Visible = $false
$form.Controls.Add($pnlFinished)

$lblFinTitle = New-Object System.Windows.Forms.Label
$lblFinTitle.Text = "MagicBridge Setup Completed!"
$lblFinTitle.Font = New-Object System.Drawing.Font("Segoe UI", 12, [System.Drawing.FontStyle]::Bold)
$lblFinTitle.ForeColor = [System.Drawing.Color]::FromArgb(0, 120, 212)
$lblFinTitle.Location = New-Object System.Drawing.Point(25, 20)
$lblFinTitle.AutoSize = $true
$pnlFinished.Controls.Add($lblFinTitle)

$lblFinBody = New-Object System.Windows.Forms.Label
$lblFinBody.Font = New-Object System.Drawing.Font("Segoe UI", 9)
$lblFinBody.ForeColor = [System.Drawing.Color]::FromArgb(50, 50, 50)
$lblFinBody.Location = New-Object System.Drawing.Point(25, 55)
$lblFinBody.Size = New-Object System.Drawing.Size(530, 140)
$pnlFinished.Controls.Add($lblFinBody)

$chkLaunchNow = New-Object System.Windows.Forms.CheckBox
$chkLaunchNow.Text = "Launch MagicBridge Settings now"
$chkLaunchNow.Location = New-Object System.Drawing.Point(25, 215)
$chkLaunchNow.Size = New-Object System.Drawing.Size(450, 24)
$chkLaunchNow.Checked = $ResolvedLaunchOnFinish
$pnlFinished.Controls.Add($chkLaunchNow)

# ----------------- Navigation Logic -----------------
$script:CurrentStep = 1

function Show-Step([int]$step) {
    $script:CurrentStep = $step
    $pnlWelcome.Visible = ($step -eq 1)
    $pnlOptions.Visible = ($step -eq 2)
    $pnlProgress.Visible = ($step -eq 3)
    $pnlFinished.Visible = ($step -eq 4)

    if ($step -eq 1) {
        $btnBack.Enabled = $false
        $btnNext.Enabled = $true
        $btnNext.Text = "Next >"
        $btnCancel.Text = "Cancel"
    } elseif ($step -eq 2) {
        $btnBack.Enabled = $true
        $btnNext.Enabled = $true
        $btnNext.Text = "Install"
        $btnCancel.Text = "Cancel"
    } elseif ($step -eq 3) {
        $btnBack.Enabled = $false
        $btnNext.Enabled = $false
        $btnCancel.Enabled = $false
    } elseif ($step -eq 4) {
        $btnBack.Visible = $false
        $btnNext.Visible = $false
        $btnCancel.Text = "Finish"
        $btnCancel.Enabled = $true
        $btnCancel.BackColor = [System.Drawing.Color]::FromArgb(0, 120, 212)
        $btnCancel.ForeColor = [System.Drawing.Color]::White
        $btnCancel.FlatStyle = [System.Windows.Forms.FlatStyle]::Flat
        $btnCancel.FlatAppearance.BorderSize = 0

        $lblFinBody.Text = @"
MagicBridge has been successfully installed on your computer.

The background keyboard hook daemon and settings application are configured and ready.

Installation Details:
 • Installed to: $($txtTargetDir.Text)
 • Configuration directory: %APPDATA%\MagicBridge
 • Registered in Windows Add/Remove Programs

Click Finish to complete the setup.
"@
    }
}

$btnNext.Add_Click({
    if ($script:CurrentStep -eq 1) {
        Show-Step 2
    } elseif ($script:CurrentStep -eq 2) {
        $chosenDir = $txtTargetDir.Text.Trim()
        if ([string]::IsNullOrWhiteSpace($chosenDir)) {
            [System.Windows.Forms.MessageBox]::Show(
                $form,
                "Please specify a valid installation directory.",
                "MagicBridge Setup",
                [System.Windows.Forms.MessageBoxButtons]::OK,
                [System.Windows.Forms.MessageBoxIcon]::Warning
            ) | Out-Null
            return
        }

        Show-Step 3
        $form.Refresh()

        $callback = {
            param([string]$statusMsg, [int]$percent)
            $lblProgStatus.Text = $statusMsg
            $progBar.Value = $percent
            $lstLog.Items.Add("[$percent%] $statusMsg") | Out-Null
            $lstLog.TopIndex = $lstLog.Items.Count - 1
            [System.Windows.Forms.Application]::DoEvents()
        }

        try {
            Install-MagicBridgePackage `
                -DestinationDir $chosenDir `
                -AddDesktopShortcut:$chkDesktop.Checked `
                -AddStartMenuShortcut:$chkStartMenu.Checked `
                -SetAutostart:$chkAutostart.Checked `
                -ProgressCallback $callback

            $chkLaunchNow.Checked = $chkLaunchWizard.Checked
            Show-Step 4
        } catch {
            $err = $_.Exception.Message
            $lblProgStatus.Text = "Installation failed!"
            $lstLog.Items.Add("ERROR: $err") | Out-Null
            [System.Windows.Forms.MessageBox]::Show(
                $form,
                "An error occurred during installation:`n`n$err",
                "Installation Error",
                [System.Windows.Forms.MessageBoxButtons]::OK,
                [System.Windows.Forms.MessageBoxIcon]::Error
            ) | Out-Null
            $btnCancel.Enabled = $true
            $btnCancel.Text = "Close"
        }
    }
})

$btnBack.Add_Click({
    if ($script:CurrentStep -eq 2) {
        Show-Step 1
    }
})

$btnCancel.Add_Click({
    if ($script:CurrentStep -eq 4) {
        # Finish clicked
        $finalDir = $txtTargetDir.Text.Trim()
        if ($chkLaunchNow.Checked) {
            $AgentExe = Join-Path $finalDir "magicbridge-agent.exe"
            if (Test-Path $AgentExe) {
                Start-Process -FilePath $AgentExe -WorkingDirectory $finalDir
            }
            $SettingsExe = Join-Path $finalDir "magicbridge-settings.exe"
            if (Test-Path $SettingsExe) {
                Start-Process -FilePath $SettingsExe -WorkingDirectory $finalDir
            }
        }
        $form.Close()
    } else {
        $res = [System.Windows.Forms.MessageBox]::Show(
            $form,
            "Are you sure you want to cancel MagicBridge Setup?",
            "Exit Setup",
            [System.Windows.Forms.MessageBoxButtons]::YesNo,
            [System.Windows.Forms.MessageBoxIcon]::Question
        )
        if ($res -eq [System.Windows.Forms.DialogResult]::Yes) {
            $form.Close()
        }
    }
})

# Display the wizard
[void]$form.ShowDialog()
