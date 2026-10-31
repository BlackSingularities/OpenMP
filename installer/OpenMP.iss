; Instalator OpenMP – dla bieżącego użytkownika, bez uprawnień administratora.
; Wersję podaje workflow: ISCC /DAppVersion=1.2.3 installer\OpenMP.iss

#ifndef AppVersion
  #define AppVersion "1.0.0"
#endif

[Setup]
AppId={{8C0F5E2A-4B71-4D8E-9A63-0F5B1C7D2E91}
AppName=OpenMP
AppVersion={#AppVersion}
AppVerName=OpenMP {#AppVersion}
AppPublisher=Adam Rędzikowski
AppPublisherURL=https://redzikowski.com
AppSupportURL=https://redzikowski.com
AppCopyright=Pomysł i wykonanie: Adam Rędzikowski · redzikowski.com
DefaultDirName={localappdata}\Programs\OpenMP
DefaultGroupName=OpenMP
DisableProgramGroupPage=yes
DisableDirPage=auto
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\out
OutputBaseFilename=OpenMP-Setup-{#AppVersion}
SetupIconFile=icon.ico
UninstallDisplayIcon={app}\OpenMP.exe
UninstallDisplayName=OpenMP
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=yes
ChangesAssociations=yes
VersionInfoVersion={#AppVersion}
VersionInfoDescription=OpenMP – odtwarzacz muzyki i wideo

[Languages]
Name: "pl"; MessagesFile: "compiler:Languages\Polish.isl"

[Tasks]
Name: "desktopicon"; Description: "Skrót na pulpicie"; GroupDescription: "Skróty:"
Name: "assocaudio"; Description: "Otwieraj pliki muzyczne w OpenMP"; GroupDescription: "Skojarzenia plików:"
Name: "assocvideo"; Description: "Otwieraj pliki wideo w OpenMP"; GroupDescription: "Skojarzenia plików:"; Flags: unchecked

[Files]
Source: "..\dist\OpenMP.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\libmpv-2.dll"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\OpenMP"; Filename: "{app}\OpenMP.exe"
Name: "{autodesktop}\OpenMP"; Filename: "{app}\OpenMP.exe"; Tasks: desktopicon

[Registry]
; Klasy plików i „Otwórz za pomocą”.
Root: HKA; Subkey: "Software\Classes\OpenMP.Audio"; ValueType: string; ValueData: "Plik muzyczny (OpenMP)"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\OpenMP.Audio\DefaultIcon"; ValueType: string; ValueData: "{app}\OpenMP.exe,0"
Root: HKA; Subkey: "Software\Classes\OpenMP.Audio\shell\open\command"; ValueType: string; ValueData: """{app}\OpenMP.exe"" ""%1"""
Root: HKA; Subkey: "Software\Classes\OpenMP.Audio\shell\enqueue"; ValueType: string; ValueData: "Dodaj do listy OpenMP"
Root: HKA; Subkey: "Software\Classes\OpenMP.Audio\shell\enqueue\command"; ValueType: string; ValueData: """{app}\OpenMP.exe"" --add ""%1"""
Root: HKA; Subkey: "Software\Classes\OpenMP.Video"; ValueType: string; ValueData: "Plik wideo (OpenMP)"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\OpenMP.Video\DefaultIcon"; ValueType: string; ValueData: "{app}\OpenMP.exe,0"
Root: HKA; Subkey: "Software\Classes\OpenMP.Video\shell\open\command"; ValueType: string; ValueData: """{app}\OpenMP.exe"" ""%1"""
Root: HKA; Subkey: "Software\Classes\Applications\OpenMP.exe\shell\open\command"; ValueType: string; ValueData: """{app}\OpenMP.exe"" ""%1"""; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\.mp3\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Audio"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocaudio
Root: HKA; Subkey: "Software\Classes\.flac\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Audio"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocaudio
Root: HKA; Subkey: "Software\Classes\.ogg\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Audio"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocaudio
Root: HKA; Subkey: "Software\Classes\.opus\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Audio"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocaudio
Root: HKA; Subkey: "Software\Classes\.m4a\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Audio"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocaudio
Root: HKA; Subkey: "Software\Classes\.aac\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Audio"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocaudio
Root: HKA; Subkey: "Software\Classes\.wav\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Audio"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocaudio
Root: HKA; Subkey: "Software\Classes\.aiff\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Audio"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocaudio
Root: HKA; Subkey: "Software\Classes\.wma\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Audio"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocaudio
Root: HKA; Subkey: "Software\Classes\.ape\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Audio"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocaudio
Root: HKA; Subkey: "Software\Classes\.wv\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Audio"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocaudio
Root: HKA; Subkey: "Software\Classes\.mpc\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Audio"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocaudio
Root: HKA; Subkey: "Software\Classes\.m3u\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Audio"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocaudio
Root: HKA; Subkey: "Software\Classes\.m3u8\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Audio"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocaudio
Root: HKA; Subkey: "Software\Classes\.pls\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Audio"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocaudio
Root: HKA; Subkey: "Software\Classes\.cue\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Audio"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocaudio
Root: HKA; Subkey: "Software\Classes\.mp4\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocvideo
Root: HKA; Subkey: "Software\Classes\.mkv\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocvideo
Root: HKA; Subkey: "Software\Classes\.avi\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocvideo
Root: HKA; Subkey: "Software\Classes\.mov\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocvideo
Root: HKA; Subkey: "Software\Classes\.webm\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocvideo
Root: HKA; Subkey: "Software\Classes\.wmv\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocvideo
Root: HKA; Subkey: "Software\Classes\.m4v\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocvideo
Root: HKA; Subkey: "Software\Classes\.ts\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocvideo
Root: HKA; Subkey: "Software\Classes\.flv\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocvideo
Root: HKA; Subkey: "Software\Classes\.mpg\OpenWithProgids"; ValueType: string; ValueName: "OpenMP.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocvideo

[Run]
Filename: "{app}\OpenMP.exe"; Description: "Uruchom OpenMP"; Flags: nowait postinstall skipifsilent
