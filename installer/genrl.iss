; Inno Setup 6 Script for Genrl Downloader (Windows x64)
; Bundles the Flutter desktop runner, assets, and genrl.exe core binary.

#define MyAppName "Genrl Downloader"
#define MyAppVersion "0.1.0"
#define MyAppPublisher "Genrl"
#define MyAppURL "https://github.com/genrl/Genrl-downloader"
#define MyAppExeName "genrl_downloader.exe"
#define MyCoreExeName "genrl.exe"

[Setup]
AppId={{D37F206B-0288-4A59-86BC-DF7D394A0E18}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
OutputDir=..\dist
OutputBaseFilename=GenrlDownloader-Setup-{#MyAppVersion}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
; Flutter Windows desktop bundle
Source: "..\app\build\windows\x64\runner\Release\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
; Bundled genrl core CLI & engine executable placed directly alongside runner
Source: "..\core\target\x86_64-pc-windows-msvc\release\{#MyCoreExeName}"; DestDir: "{app}"; DestName: "{#MyCoreExeName}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
