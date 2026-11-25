; Plexdio installer (Inno Setup 6.3+ or 7). Built by `./build.sh release` (Inno Setup 7 under Wine):
;   ISCC.exe /DSourceDir=<folder with plexdio.exe, README.txt, LICENSE.txt> /DAppVersion=0.1.0 /O<output folder> installer\plexdio.iss
; Mirrors Screvia's installer (KruxSYS sister project): per-user install by default (no admin prompt, silent updates
; work), "for all users" offered when elevated; Start menu shortcut, optional desktop icon, uninstaller; user data is
; kept on uninstall.

#ifndef SourceDir
  #define SourceDir "..\dist\installer-src"
#endif
#ifndef AppVersion
  #error Pass the version: /DAppVersion=x.y.z (./build.sh release reads it from Cargo.toml)
#endif

#define AppName "Plexdio"
#define AppExe "plexdio.exe"
#define AppPublisher "KruxSYS"
#define PublisherUrl "https://kruxsys.com/projects/plexdio/"
#define AppUrl "https://plexdio.kruxsys.com"

[Setup]
AppId={{9AE83232-8D6D-49B1-B1C5-1304AC3F9881}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#PublisherUrl}
AppSupportURL={#AppUrl}/docs
AppUpdatesURL={#AppUrl}/download
AppCopyright=© 2026 KruxSYS
; {autopf} is %LOCALAPPDATA%\Programs for a per-user install, Program Files for all users.
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog commandline
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputBaseFilename={#AppName}-{#AppVersion}-win-x64-setup
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
WizardImageFile=wizard-100.bmp,wizard-200.bmp
WizardSmallImageFile=wizard-small-100.bmp,wizard-small-200.bmp
VersionInfoCompany={#AppPublisher}
VersionInfoCopyright=© 2026 KruxSYS
VersionInfoProductName={#AppName}
VersionInfoVersion={#AppVersion}
VersionInfoDescription={#AppName} setup
; Plexdio may be running (often only as a tray icon). The Restart Manager closes it before the files are replaced:
; interactively Setup asks first, updates pass /CLOSEAPPLICATIONS. Plexdio does not register for Restart Manager
; restarts, so [Run] below starts the new version after a silent update instead.
CloseApplications=force
RestartApplications=no
UsePreviousAppDir=yes
UsePreviousTasks=yes

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"

[Messages]
; Shown after a complete uninstall (UninstalledMost only appears when files could not be removed).
en.UninstalledAll=Plexdio has been removed.%n%nYour settings, library cache and downloads were kept in %APPDATA%\Plexdio and %LOCALAPPDATA%\Plexdio, and the Plex sign-in in the Windows Credential Manager. Delete those folders to remove them too.
en.UninstalledMost=Plexdio has been removed, but some files could not be deleted.%n%nYour settings, library cache and downloads were kept in %APPDATA%\Plexdio and %LOCALAPPDATA%\Plexdio.

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\README.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\LICENSE.txt"; DestDir: "{app}"; Flags: ignoreversion
; The marker tells Plexdio it was installed: settings stay in the Windows profile and updates run this installer.
Source: "installed"; DestDir: "{app}"; Flags: ignoreversion

[InstallDelete]
; A portable marker from a copied-in portable zip would move settings to {app}\Data.
Type: files; Name: "{app}\portable"
; Left behind by an interrupted portable-style self-update.
Type: files; Name: "{app}\plexdio.exe.old"

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"; Comment: "Music player for Plex"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
; Interactive install: offer to start Plexdio. Silent update: start it again unconditionally.
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent
Filename: "{app}\{#AppExe}"; Flags: nowait; Check: WizardSilent

[UninstallRun]
; Removes the start-with-Windows entry (HKCU Run), which lives outside {app}.
Filename: "{app}\{#AppExe}"; Parameters: "--uninstall"; Flags: runhidden waituntilterminated; RunOnceId: "PlexdioUninstall"

[UninstallDelete]
Type: files; Name: "{app}\installed"
Type: files; Name: "{app}\plexdio.exe.old"
