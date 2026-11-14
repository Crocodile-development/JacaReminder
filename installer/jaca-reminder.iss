; Installer of Jaca Reminder, made with Inno Setup (https://jrsoftware.org/isinfo.php).
;
; How to build the installer:
;   1. cargo build --release                      (makes target\release\jaca-reminder.exe)
;   2. ISCC.exe installer\jaca-reminder.iss       (add /DAppVersion=1.2.3 to name another version)
;   3. The installer is written to dist\JacaReminder-Setup-<version>.exe
;
; The program is one file with everything inside (the C runtime, the water sound and the logo), so
; the installer copies a single exe and nothing else has to be installed on the computer.
;
; The installer works for the current user only (no administrator rights). It shows, in English or
; Brazilian Portuguese depending on the language of Windows, one choice: a shortcut on the desktop.
; Whether the program starts with Windows is chosen inside the program (Settings > General), which
; keeps that option in one place.
;
; The reminders, the history and the sounds live in %APPDATA%\JacaReminder. They are kept when the
; program is updated or uninstalled, so a new installation finds the reminders again.

#define AppName "Jaca Reminder"
#define AppExe "jaca-reminder.exe"
#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif

[Setup]
; Identifies this program to Windows, so a new installer replaces the installed version.
; The doubled "{" is how a literal "{" is written in this file.
AppId={{2D4DC6AC-5572-4BA2-8FEF-D0748FD48C8A}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=Crocodile development
; "autopf" is "Program Files"; without administrator rights it becomes the folder of programs of
; the current user.
DefaultDirName={autopf}\{#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
; Not just "setup": Windows treats every program called setup.exe in a special way that can be
; abused to make it load a harmful library.
OutputBaseFilename=JacaReminder-Setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\{#AppExe}
; The logo of the installer file itself and of its window.
SetupIconFile=..\assets\app-icon.ico
; Never asks. It uses the language of Windows when it is listed below (Portuguese or English),
; and English, the first one listed, for any other language.
ShowLanguageDialog=no

[Languages]
; The installer picks the language of Windows when it is one of these two.
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"

[CustomMessages]
; Texts that are not part of Inno Setup itself, one set per language. Each one must be on one line.
english.DesktopIconTask=Create a shortcut on the desktop
brazilianportuguese.DesktopIconTask=Criar um atalho na área de trabalho

[Tasks]
; The choices of the "Select Additional Tasks" page.
Name: "desktopicon"; Description: "{cm:DesktopIconTask}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "..\target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
; Only when the box is checked: the shortcut on the desktop (it takes the logo of the program).
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Registry]
; The "start with Windows" value the program writes in Settings. It is removed when the program
; is uninstalled, so Windows does not try to start a program that is gone.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueName: "JacaReminder"; Flags: dontcreatekey uninsdeletevalue

[Run]
; Offers to open the program when the installation ends. The program starts its background part
; (the one that shows the reminders) by itself.
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent

[Code]
// Small programs written in Pascal, the language of Inno Setup scripts.

// The program keeps running in the background (tray icon and scheduler), and a running exe cannot
// be replaced or deleted, so it is closed before an update and before the uninstall.
procedure CloseRunningProgram;
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM {#AppExe}', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

// Called before the files are copied. An empty answer means "go on".
function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  CloseRunningProgram;
  Result := '';
end;

// Called when the uninstall starts. True means "go on".
function InitializeUninstall: Boolean;
begin
  CloseRunningProgram;
  Result := True;
end;
