<#
.SYNOPSIS
    Generates WinGet and Scoop package manifests for a given MagicBridge release.

.DESCRIPTION
    Calculates the SHA256 of the distribution zip and updates the manifest files in packaging/
    ready for submission to microsoft/winget-pkgs or Scoop buckets.

.PARAMETER Version
    The release version (e.g. "0.2.0"). If omitted, reads from Cargo.toml.

.PARAMETER ArchivePath
    Path to the release ZIP. Defaults to dist\MagicBridge-v<Version>-windows-x64.zip.

.EXAMPLE
    .\scripts\generate-manifests.ps1
    .\scripts\generate-manifests.ps1 -Version "0.2.0"
#>

[CmdletBinding()]
param(
    [string]$Version,
    [string]$ArchivePath
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (Resolve-Path (Join-Path $ScriptDir "..")).Path

# 1. Resolve version
if (-not $Version) {
    $CargoToml = Join-Path $RepoRoot "Cargo.toml"
    if (Test-Path $CargoToml) {
        $content = Get-Content $CargoToml -Raw
        if ($content -match '\[workspace\.package\][\s\S]*?version\s*=\s*"([^"]+)"') {
            $Version = $matches[1].Trim()
        }
    }
}
if (-not $Version) { $Version = "0.2.0" }

Write-Host "Generating package manifests for MagicBridge v$Version..." -ForegroundColor Cyan

# 2. Resolve Archive & Calculate Hash
if (-not $ArchivePath) {
    $ArchivePath = Join-Path $RepoRoot "dist\MagicBridge-v$Version-windows-x64.zip"
}

if (-not (Test-Path $ArchivePath)) {
    throw "Archive not found at '$ArchivePath'. Please run scripts\package-release.ps1 first."
}

$Hash = (Get-FileHash -Path $ArchivePath -Algorithm SHA256).Hash.ToUpper()
Write-Host "Archive: $ArchivePath" -ForegroundColor Gray
Write-Host "SHA256:  $Hash" -ForegroundColor Green

# 3. Output WinGet Manifests
$WingetDir = Join-Path $RepoRoot "packaging\winget\manifests\d\DooperDrew\MagicBridge\$Version"
New-Item -ItemType Directory -Path $WingetDir -Force | Out-Null

$VersionYaml = @"
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.version.1.9.0.schema.json
PackageIdentifier: DooperDrew.MagicBridge
PackageVersion: $Version
DefaultLocale: en-US
ManifestType: version
ManifestVersion: 1.9.0
"@

$InstallerYaml = @"
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.installer.1.9.0.schema.json
PackageIdentifier: DooperDrew.MagicBridge
PackageVersion: $Version
InstallerType: zip
NestedInstallerType: portable
NestedInstallerFiles:
  - RelativeFilePath: magicbridge-agent.exe
    PortableCommandAlias: magicbridge-agent
  - RelativeFilePath: magicbridge-settings.exe
    PortableCommandAlias: magicbridge
Installers:
  - Architecture: x64
    InstallerUrl: https://github.com/DooperDrew/magicbridge/releases/download/v$Version/MagicBridge-v$Version-windows-x64.zip
    InstallerSha256: $Hash
    UpgradeBehavior: install
ManifestType: installer
ManifestVersion: 1.9.0
"@

$LocaleYaml = @"
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.defaultLocale.1.9.0.schema.json
PackageIdentifier: DooperDrew.MagicBridge
PackageVersion: $Version
PackageLocale: en-US
Publisher: DooperDrew
PublisherUrl: https://github.com/DooperDrew
PublisherSupportUrl: https://github.com/DooperDrew/magicbridge/issues
Author: DooperDrew
PackageName: MagicBridge
PackageUrl: https://github.com/DooperDrew/magicbridge
License: MIT
LicenseUrl: https://github.com/DooperDrew/magicbridge/blob/main/LICENSE
Copyright: Copyright (c) 2026 DooperDrew
ShortDescription: Native, ultra-lightweight Windows utility making Apple Magic Keyboards behave as first-class Windows keyboards.
Description: |
  MagicBridge is a native, ultra-lightweight Windows 10/11 x64 utility that makes the Apple Magic Keyboard
  with Numeric Keypad (model A1843) feel right at home on Windows. Features user-mode Fn layer, top-row media keys
  vs. F-keys toggling, modifier presets, direct HID battery reporting, and zero background idle CPU.
Moniker: magicbridge
Tags:
  - keyboard
  - apple
  - magic-keyboard
  - remap
  - hotkey
  - battery
  - rust
  - windows11
ReleaseNotesUrl: https://github.com/DooperDrew/magicbridge/releases/tag/v$Version
ManifestType: defaultLocale
ManifestVersion: 1.9.0
"@

[System.IO.File]::WriteAllText((Join-Path $WingetDir "DooperDrew.MagicBridge.yaml"), $VersionYaml, [System.Text.Encoding]::UTF8)
[System.IO.File]::WriteAllText((Join-Path $WingetDir "DooperDrew.MagicBridge.installer.yaml"), $InstallerYaml, [System.Text.Encoding]::UTF8)
[System.IO.File]::WriteAllText((Join-Path $WingetDir "DooperDrew.MagicBridge.locale.en-US.yaml"), $LocaleYaml, [System.Text.Encoding]::UTF8)
Write-Host "  -> WinGet manifests written to: $WingetDir" -ForegroundColor Green

# 4. Output Scoop Manifest
$ScoopDir = Join-Path $RepoRoot "packaging\scoop"
New-Item -ItemType Directory -Path $ScoopDir -Force | Out-Null

$ScoopJson = @"
{
    "version": "$Version",
    "description": "Native, ultra-lightweight Windows utility making Apple Magic Keyboards behave as first-class Windows keyboards.",
    "homepage": "https://github.com/DooperDrew/magicbridge",
    "license": "MIT",
    "architecture": {
        "64bit": {
            "url": "https://github.com/DooperDrew/magicbridge/releases/download/v$Version/MagicBridge-v$Version-windows-x64.zip",
            "hash": "$Hash"
        }
    },
    "bin": [
        "magicbridge-agent.exe",
        [
            "magicbridge-settings.exe",
            "magicbridge"
        ]
    ],
    "shortcuts": [
        [
            "magicbridge-settings.exe",
            "MagicBridge Settings"
        ]
    ],
    "checkver": "github",
    "autoupdate": {
        "architecture": {
            "64bit": {
                "url": "https://github.com/DooperDrew/magicbridge/releases/download/v`$version/MagicBridge-v`$version-windows-x64.zip"
            }
        }
    }
}
"@

[System.IO.File]::WriteAllText((Join-Path $ScoopDir "magicbridge.json"), $ScoopJson, [System.Text.Encoding]::UTF8)
Write-Host "  -> Scoop manifest written to: $(Join-Path $ScoopDir 'magicbridge.json')" -ForegroundColor Green

Write-Host "`nAll package distribution manifests generated successfully!" -ForegroundColor Cyan
