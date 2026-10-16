; Inno Setup script for S76 Fan Control.
; Built by .github/workflows/release.yml:
;   iscc /DAppVersion=1.2.3 installer\S76FanControl.iss
; Expects `cargo build --release` to have run first.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#define AppName "S76 Fan Control"
#define AppExe "S76FanControl.exe"
#define TaskName "S76FanControl"
#define RepoUrl "https://github.com/congluuit/NANOSystem76Fan"

[Setup]
AppId={{7B1E5C2A-4F0D-4C8B-9A61-2D76F0A5C3E1}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=NANOSystem76Fan contributors
AppPublisherURL={#RepoUrl}
AppSupportURL={#RepoUrl}/issues
AppUpdatesURL={#RepoUrl}/releases
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
LicenseFile=..\LICENSE
OutputDir=..\dist
OutputBaseFilename=S76FanControl-{#AppVersion}-setup
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
Compression=lzma2/ultra
SolidCompression=yes
WizardStyle=modern
; The app is closed through `--quit` (see [Code]) so it can hand the fans back to the EC.
CloseApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "autostart"; Description: "Start with Windows (minimized to the tray)"; GroupDescription: "Options:"
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\vendor\inpout\inpoutx64.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion
Source: "..\vendor\inpout\LICENSE.txt"; DestDir: "{app}"; DestName: "LICENSE-InpOut.txt"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
; Same logon task the app creates from Settings → "Start with Windows".
Filename: "{sys}\schtasks.exe"; \
  Parameters: "/Create /F /TN ""{#TaskName}"" /SC ONLOGON /RL HIGHEST /DELAY 0000:10 /TR ""\""{app}\{#AppExe}\"" --minimized"""; \
  Flags: runhidden waituntilterminated; Tasks: autostart
Filename: "{app}\{#AppExe}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{app}\{#AppExe}"; Parameters: "--quit"; Flags: runhidden waituntilterminated; RunOnceId: "QuitApp"
Filename: "{sys}\schtasks.exe"; Parameters: "/Delete /F /TN ""{#TaskName}"""; Flags: runhidden waituntilterminated; RunOnceId: "DeleteTask"

[Code]
// Before upgrading, ask a running instance to exit cleanly (fans back to auto).
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Exe: String;
  ResultCode: Integer;
begin
  Exe := ExpandConstant('{app}\{#AppExe}');
  if FileExists(Exe) then
    Exec(Exe, '--quit', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Result := '';
end;
