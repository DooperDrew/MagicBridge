; Inno Setup Script for MagicBridge
; Zero-elevation Windows installer for Apple Magic Keyboard with Numeric Keypad (A1843)
#define MyAppName "MagicBridge"
#define MyAppVersion "0.2.0"
#define MyAppPublisher "MagicBridge Contributors"
#define MyAppURL "https://github.com/DooperDrew/magicbridge"
#define MyAppExeName "magicbridge-settings.exe"
#define MyAgentExeName "magicbridge-agent.exe"

[Setup]
AppId={{B523AE21-0D34-4C2E-8E92-31FE66A487B2}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={localappdata}\Programs\MagicBridge
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
; Zero Administrator elevation required (per-user install into %LOCALAPPDATA%)
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=..\dist
OutputBaseFilename=MagicBridge-v{#MyAppVersion}-Setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
CloseApplicationsFilter={#MyAgentExeName},{#MyAppExeName}
UninstallDisplayIcon={app}\{#MyAgentExeName}
VersionInfoVersion={#MyAppVersion}
VersionInfoCompany={#MyAppPublisher}
VersionInfoDescription=MagicBridge Setup (Apple Magic Keyboard for Windows)

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "autostart"; Description: "Start MagicBridge automatically at Windows sign-in"; GroupDescription: "Startup options:"

[Files]
Source: "..\target\release\{#MyAgentExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\crates\settings\locales\*"; DestDir: "{app}\locales"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\docs\user-guide.md"; DestDir: "{app}"; Flags: ignoreversion; DestName: "user-guide.md"
Source: "uninstall.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "setup-wizard.ps1"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\MagicBridge Settings"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; Comment: "Configure Apple Magic Keyboard mappings and profiles"
Name: "{group}\MagicBridge Agent"; Filename: "{app}\{#MyAgentExeName}"; WorkingDir: "{app}"; Comment: "Background keyboard remap hook and battery monitor"
Name: "{group}\Uninstall MagicBridge"; Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\uninstall.ps1"""; Comment: "Uninstall MagicBridge"
Name: "{autodesktop}\MagicBridge Settings"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; Tasks: desktopicon; Comment: "MagicBridge Settings"

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "MagicBridge"; ValueData: """{app}\{#MyAgentExeName}"""; Tasks: autostart; Flags: uninsdeletevalue

[Run]
Filename: "{app}\{#MyAgentExeName}"; Description: "Start MagicBridge background agent"; Flags: nowait postinstall skipifsilent
Filename: "{app}\{#MyAppExeName}"; Description: "Launch Initial Setup Wizard / Settings"; Flags: nowait postinstall skipifsilent checked

[UninstallDelete]
Type: files; Name: "{app}\*.log"
Type: dirifempty; Name: "{app}\locales"
Type: dirifempty; Name: "{app}"

[Code]
// Automatically terminate any running MagicBridge instances prior to installation
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ResultCode: Integer;
begin
  Exec('taskkill.exe', '/F /IM magicbridge-agent.exe /IM magicbridge-settings.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Result := '';
end;

// Stop running instances prior to uninstallation
function InitializeUninstall(): Boolean;
var
  ResultCode: Integer;
begin
  Exec('taskkill.exe', '/F /IM magicbridge-agent.exe /IM magicbridge-settings.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Result := True;
end;
