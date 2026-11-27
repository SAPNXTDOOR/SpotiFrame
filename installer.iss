[Setup]
AppId={{8F3B9C1A-4D2E-4A7B-9E1C-5A6B7C8D9E0F}
AppName=SpotiFrame
AppVersion=0.1.1
AppPublisher=Saptarshi Mallick
DefaultDirName={autopf}\SpotiFrame
DefaultGroupName=SpotiFrame
DisableDirPage=no
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
SetupIconFile=assets\icon.ico
UninstallDisplayIcon={app}\SpotiFrame.exe
OutputDir=dist
OutputBaseFilename=SpotiFrame-Setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "startmenu"; Description: "Create a &Start Menu shortcut"; GroupDescription: "Start Menu shortcuts:"; Flags: unchecked
Name: "startmenu_fs"; Description: "Create a Start Menu shortcut (&Fullscreen)"; GroupDescription: "Start Menu shortcuts:"; Flags: unchecked
Name: "desktop"; Description: "Create a &Desktop shortcut"; GroupDescription: "Desktop shortcuts:"; Flags: unchecked
Name: "desktop_fs"; Description: "Create a Desktop shortcut (F&ullscreen)"; GroupDescription: "Desktop shortcuts:"; Flags: unchecked

[Files]
Source: "target\release\Final\SpotiFrame.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "target\release\Final\assets\*"; DestDir: "{app}\assets"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\SpotiFrame\SpotiFrame"; Filename: "{app}\SpotiFrame.exe"; WorkingDir: "{app}"; Tasks: startmenu
Name: "{autoprograms}\SpotiFrame\SpotiFrame (Fullscreen)"; Filename: "{app}\SpotiFrame.exe"; Parameters: "-fullscreen"; WorkingDir: "{app}"; Tasks: startmenu_fs
Name: "{autodesktop}\SpotiFrame"; Filename: "{app}\SpotiFrame.exe"; WorkingDir: "{app}"; Tasks: desktop
Name: "{autodesktop}\SpotiFrame (Fullscreen)"; Filename: "{app}\SpotiFrame.exe"; Parameters: "-fullscreen"; WorkingDir: "{app}"; Tasks: desktop_fs

[Run]
Filename: "{app}\SpotiFrame.exe"; Description: "Launch SpotiFrame"; Flags: nowait postinstall skipifsilent 
