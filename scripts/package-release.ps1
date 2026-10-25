<#
.SYNOPSIS
    Packages MagicBridge into a portable release ZIP archive for Windows x64.

.DESCRIPTION
    Builds the MagicBridge Cargo workspace in release mode, validates that all
    required binary and resource artifacts exist, stages them into a clean bundle
    (including setup-wizard.ps1, setup.cmd, install.ps1, uninstall.ps1, and locales),
    creates a portable zip archive in the output directory (default: dist/), and
    optionally compiles an Inno Setup installer executable if Inno Setup is detected.

.PARAMETER Version
    The release version string (e.g. "0.1.0"). If omitted, the version is
    extracted from the root Cargo.toml workspace.package.version field.

.PARAMETER OutputDir
    Destination folder for the final release ZIP (default: "dist").

.PARAMETER SkipBuild
    If set, skips running 'cargo build --workspace --release' and uses existing
    binaries in target/release.

.EXAMPLE
    .\scripts\package-release.ps1
    .\scripts\package-release.ps1 -Version "0.1.0" -OutputDir "dist"
    .\scripts\package-release.ps1 -SkipBuild
#>

[CmdletBinding()]
param (
    [string]$Version,
    [string]$OutputDir = "dist",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

# Ensure script runs from the repository root
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RepoRoot = Resolve-Path (Join-Path $ScriptDir "..")
Set-Location $RepoRoot

Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "       MagicBridge Release Packaging Tool         " -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "Repository root: $RepoRoot"

# Ensure cargo is available in PATH
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $CargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    if (Test-Path (Join-Path $CargoBin "cargo.exe")) {
        Write-Host "Adding $CargoBin to PATH..." -ForegroundColor Yellow
        $env:PATH = "$CargoBin;$env:PATH"
    } else {
        throw "cargo was not found on PATH and was not located at $CargoBin\cargo.exe"
    }
}

# Resolve version
if (-not $Version) {
    $CargoTomlPath = Join-Path $RepoRoot "Cargo.toml"
    if (Test-Path $CargoTomlPath) {
        $CargoTomlContent = Get-Content $CargoTomlPath -Raw
        if ($CargoTomlContent -match '\[workspace\.package\][\s\S]*?version\s*=\s*"([^"]+)"') {
            $Version = $matches[1]
            Write-Host "Extracted version from Cargo.toml: v$Version" -ForegroundColor Green
        }
    }
    if (-not $Version) {
        $Version = "0.1.0"
        Write-Host "Defaulting version to: v$Version" -ForegroundColor Yellow
    }
}

# 1. Build workspace in release mode
if (-not $SkipBuild) {
    Write-Host "`n[1/5] Compiling workspace in release mode..." -ForegroundColor Cyan
    $BuildArgs = @("build", "--workspace", "--release")
    & cargo @BuildArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo release build failed with exit code $LASTEXITCODE"
    }
    Write-Host "Release compilation succeeded." -ForegroundColor Green
} else {
    Write-Host "`n[1/5] Skipping build as requested (-SkipBuild)..." -ForegroundColor Yellow
}

# 2. Verify release binaries exist
Write-Host "`n[2/5] Verifying release binaries and assets..." -ForegroundColor Cyan
$AgentExe = Join-Path $RepoRoot "target\release\magicbridge-agent.exe"
$SettingsExe = Join-Path $RepoRoot "target\release\magicbridge-settings.exe"
$LocalesSrc = Join-Path $RepoRoot "crates\settings\locales"
$ReadmeSrc = Join-Path $RepoRoot "README.md"
$LicenseSrc = Join-Path $RepoRoot "LICENSE"
$UserGuideSrc = Join-Path $RepoRoot "docs\user-guide.md"

if (-not (Test-Path $AgentExe)) {
    throw "Required binary missing: $AgentExe"
}
if (-not (Test-Path $SettingsExe)) {
    throw "Required binary missing: $SettingsExe"
}
if (-not (Test-Path $LocalesSrc)) {
    throw "Locales directory missing: $LocalesSrc"
}
if (-not (Test-Path $ReadmeSrc)) {
    throw "README.md missing: $ReadmeSrc"
}
if (-not (Test-Path $LicenseSrc)) {
    throw "LICENSE missing: $LicenseSrc"
}

Write-Host "  - Found agent binary: $AgentExe" -ForegroundColor Green
Write-Host "  - Found settings binary: $SettingsExe" -ForegroundColor Green
Write-Host "  - Found translations: $LocalesSrc" -ForegroundColor Green
Write-Host "  - Found documentation and license." -ForegroundColor Green

# 3. Create clean staging directory
Write-Host "`n[3/5] Assembling staging directory..." -ForegroundColor Cyan
$AbsoluteOutputDir = Join-Path $RepoRoot $OutputDir
if (-not (Test-Path $AbsoluteOutputDir)) {
    New-Item -ItemType Directory -Path $AbsoluteOutputDir -Force | Out-Null
}

$ArchiveBaseName = "MagicBridge-v$Version-windows-x64"
$StagingDir = Join-Path $AbsoluteOutputDir "staging\$ArchiveBaseName"

if (Test-Path $StagingDir) {
    Remove-Item -Path $StagingDir -Recurse -Force
}
New-Item -ItemType Directory -Path $StagingDir -Force | Out-Null

# Copy binaries
Copy-Item -Path $AgentExe -Destination (Join-Path $StagingDir "magicbridge-agent.exe") -Force
Copy-Item -Path $SettingsExe -Destination (Join-Path $StagingDir "magicbridge-settings.exe") -Force

# Copy locales
$StagingLocales = Join-Path $StagingDir "locales"
New-Item -ItemType Directory -Path $StagingLocales -Force | Out-Null
Copy-Item -Path (Join-Path $LocalesSrc "en.toml") -Destination $StagingLocales -Force
Copy-Item -Path (Join-Path $LocalesSrc "it.toml") -Destination $StagingLocales -Force

# Copy documentation & license
Copy-Item -Path $ReadmeSrc -Destination $StagingDir -Force
Copy-Item -Path $LicenseSrc -Destination $StagingDir -Force
if (Test-Path $UserGuideSrc) {
    Copy-Item -Path $UserGuideSrc -Destination $StagingDir -Force
}

# Copy installer and setup scripts
$SetupWizardScript = Join-Path $RepoRoot "scripts\setup-wizard.ps1"
$SetupCmd = Join-Path $RepoRoot "scripts\setup.cmd"
$InstallScript = Join-Path $RepoRoot "scripts\install.ps1"
$UninstallScript = Join-Path $RepoRoot "scripts\uninstall.ps1"
$InstallerIss = Join-Path $RepoRoot "scripts\installer.iss"

if (Test-Path $SetupWizardScript) {
    Copy-Item -Path $SetupWizardScript -Destination $StagingDir -Force
}
if (Test-Path $SetupCmd) {
    Copy-Item -Path $SetupCmd -Destination $StagingDir -Force
}
if (Test-Path $InstallScript) {
    Copy-Item -Path $InstallScript -Destination $StagingDir -Force
}
if (Test-Path $UninstallScript) {
    Copy-Item -Path $UninstallScript -Destination $StagingDir -Force
}
if (Test-Path $InstallerIss) {
    Copy-Item -Path $InstallerIss -Destination $StagingDir -Force
}

Write-Host "Staging folder populated at: $StagingDir" -ForegroundColor Green

# 4. Create ZIP package
Write-Host "`n[4/5] Creating ZIP package..." -ForegroundColor Cyan
$ZipFilePath = Join-Path $AbsoluteOutputDir "$ArchiveBaseName.zip"
if (Test-Path $ZipFilePath) {
    Remove-Item -Path $ZipFilePath -Force
}

Compress-Archive -Path "$StagingDir\*" -DestinationPath $ZipFilePath -CompressionLevel Optimal

# Cleanup staging directory
Remove-Item -Path (Join-Path $AbsoluteOutputDir "staging") -Recurse -Force

$ZipItem = Get-Item $ZipFilePath
$ZipSizeMB = [math]::Round($ZipItem.Length / 1MB, 2)

# 5. Optional Inno Setup compilation
Write-Host "`n[5/5] Checking for Inno Setup compiler (iscc.exe)..." -ForegroundColor Cyan
$CandidateIscc = @(
    (Get-Command iscc.exe -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -ErrorAction SilentlyContinue),
    "${env:ProgramFiles(x86)}\Inno Setup 6\iscc.exe",
    "$env:ProgramFiles\Inno Setup 6\iscc.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 5\iscc.exe",
    "$env:ProgramFiles\Inno Setup 5\iscc.exe"
)
$IsccExe = $CandidateIscc | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1

if ($IsccExe -and (Test-Path $InstallerIss)) {
    Write-Host "  Found Inno Setup compiler: $IsccExe" -ForegroundColor Green
    Write-Host "  Compiling Inno Setup installer..." -ForegroundColor Cyan
    & $IsccExe "/DMyAppVersion=$Version" $InstallerIss
    if ($LASTEXITCODE -eq 0) {
        $SetupExe = Join-Path $AbsoluteOutputDir "MagicBridge-v$Version-Setup.exe"
        if (Test-Path $SetupExe) {
            $SetupItem = Get-Item $SetupExe
            $SetupSizeMB = [math]::Round($SetupItem.Length / 1MB, 2)
            Write-Host "  Built Inno Setup installer: $SetupExe ($SetupSizeMB MB)" -ForegroundColor Green
        }
    } else {
        Write-Warning "Inno Setup compilation exited with code $LASTEXITCODE"
    }
} else {
    Write-Host "  Inno Setup compiler (iscc.exe) not detected on PATH or Program Files." -ForegroundColor Gray
    Write-Host "  (Setup wizard script 'setup-wizard.ps1' is bundled inside the release ZIP)." -ForegroundColor Gray
}

Write-Host "`n==================================================" -ForegroundColor Green
Write-Host "       RELEASE PACKAGING COMPLETE SUCCESS         " -ForegroundColor Green
Write-Host "==================================================" -ForegroundColor Green
Write-Host "Archive:  $ZipFilePath" -ForegroundColor Yellow
Write-Host "Size:     $ZipSizeMB MB ($($ZipItem.Length) bytes)" -ForegroundColor Yellow
Write-Host "Contents:"
[System.IO.Compression.ZipFile]::OpenRead($ZipFilePath).Entries | ForEach-Object {
    Write-Host "  - $($_.FullName) ($([math]::Round($_.Length / 1KB, 1)) KB)" -ForegroundColor Gray
}

# 6. Auto-generate WinGet and Scoop package manifests
$GenerateManifestsScript = Join-Path $ScriptDir "generate-manifests.ps1"
if (Test-Path $GenerateManifestsScript) {
    Write-Host "`n[6/6] Generating distribution manifests (WinGet & Scoop)..." -ForegroundColor Cyan
    & powershell -ExecutionPolicy Bypass -File $GenerateManifestsScript -Version $Version -ArchivePath $ZipFilePath
}

Write-Host ""
