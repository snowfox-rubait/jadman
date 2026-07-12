[Setup]
AppName=JADMan
AppVersion=0.1.9
DefaultDirName={userpf}\JADMan
DefaultGroupName=JADMan
UninstallDisplayIcon={app}\jadm-tui.exe
Compression=lzma2
SolidCompression=yes
OutputDir=dist
OutputBaseFilename=JADMan_Setup
PrivilegesRequired=lowest

[Files]
Source: "target\x86_64-pc-windows-msvc\release\jadm-daemon.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "target\x86_64-pc-windows-msvc\release\jadm-tui.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "scripts\download_deps.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "extension\chrome\*"; DestDir: "{app}\extension\chrome"; Flags: recursesubdirs createallsubdirs ignoreversion
Source: "extension\firefox\*"; DestDir: "{app}\extension\firefox"; Flags: recursesubdirs createallsubdirs ignoreversion

[Tasks]
Name: "download_ytdlp"; Description: "Download and install yt-dlp (recommended)"; GroupDescription: "Optional Dependencies:"
Name: "download_aria2"; Description: "Download and install aria2c (recommended)"; GroupDescription: "Optional Dependencies:"
Name: "download_ffmpeg"; Description: "Download and install ffmpeg (recommended for merging streams)"; GroupDescription: "Optional Dependencies:"

[Registry]
; Native Messaging for Chrome
Root: HKCU; Subkey: "Software\Google\Chrome\NativeMessagingHosts\com.jadm.jadm"; ValueType: string; ValueValue: "{app}\com.jadm.jadm.json"; Flags: uninsdeletekey
; Native Messaging for Firefox
Root: HKCU; Subkey: "Software\Mozilla\NativeMessagingHosts\com.jadm.jadm"; ValueType: string; ValueValue: "{app}\com.jadm.jadm.json"; Flags: uninsdeletekey
; Force Install Extension for Google Chrome (if published in Chrome Store)
Root: HKCU; Subkey: "Software\Policies\Google\Chrome\ExtensionInstallForcelist"; ValueType: string; ValueName: "1"; ValueValue: "ipiefkjcicogeoepimgebinafoelhbhd;https://clients2.google.com/service/update2/crx"; Flags: uninsdeletevalue

[Run]
Filename: "powershell.exe"; Parameters: "-ExecutionPolicy Bypass -File ""{app}\download_deps.ps1"" -InstallDir ""{app}"" -Ytdlp"; Tasks: download_ytdlp; Flags: runhidden; StatusMsg: "Downloading yt-dlp..."
Filename: "powershell.exe"; Parameters: "-ExecutionPolicy Bypass -File ""{app}\download_deps.ps1"" -InstallDir ""{app}"" -Aria2"; Tasks: download_aria2; Flags: runhidden; StatusMsg: "Downloading aria2c..."
Filename: "powershell.exe"; Parameters: "-ExecutionPolicy Bypass -File ""{app}\download_deps.ps1"" -InstallDir ""{app}"" -Ffmpeg"; Tasks: download_ffmpeg; Flags: runhidden; StatusMsg: "Downloading ffmpeg & ffprobe..."

; Launch Daemon in background after installation
Filename: "{app}\jadm-daemon.exe"; Flags: nowait postinstall runhidden

; Summon browser with extension setup pages
Filename: "https://chromewebstore.google.com/detail/ipiefkjcicogeoepimgebinafoelhbhd"; Description: "Add JADMan Extension to Google Chrome"; Flags: postinstall shellexec
Filename: "https://addons.mozilla.org/en-US/firefox/addon/jadman/"; Description: "Add JADMan Extension to Mozilla Firefox"; Flags: postinstall shellexec unchecked

[Code]
const
  EnvironmentKey = 'Environment';

// Helper to add folder to user PATH variable
procedure AddToPath(PathToAdd: string);
var
  CurrentPath: string;
begin
  if RegQueryStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', CurrentPath) then
  begin
    if Pos(PathToAdd, CurrentPath) = 0 then
    begin
      if (CurrentPath <> '') and (CurrentPath[Length(CurrentPath)] <> ';') then
        CurrentPath := CurrentPath + ';';
      RegWriteStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', CurrentPath + PathToAdd);
    end;
  end;
end;

// Helper to remove folder from user PATH variable
procedure RemoveFromPath(PathToRemove: string);
var
  CurrentPath: string;
  PosIndex: Integer;
begin
  if RegQueryStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', CurrentPath) then
  begin
    PosIndex := Pos(PathToRemove, CurrentPath);
    if PosIndex > 0 then
    begin
      Delete(CurrentPath, PosIndex, Length(PathToRemove));
      // Clean up double semicolons if they occur
      StringChangeEx(CurrentPath, ';;', ';', True);
      RegWriteStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', CurrentPath);
    end;
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  JsonContent: String;
  JsonFile: String;
  AppDir: String;
  EscapedAppDir: String;
begin
  if CurStep = ssPostInstall then
  begin
    AppDir := ExpandConstant('{app}');
    
    // Add to PATH environment variable
    AddToPath(AppDir);
    AddToPath(AppDir + '\bin');
    
    // Broadcast environment change notification to system
    SendBroadcastMessage($001A, 0, 'Environment'); // WM_SETTINGCHANGE

    // Write Native Messaging manifest dynamically with escaped double backslashes
    EscapedAppDir := AppDir;
    StringChangeEx(EscapedAppDir, '\', '\\', True);
    
    JsonContent := 
      '{' + #13#10 +
      '  "name": "com.jadm.jadm",' + #13#10 +
      '  "description": "JADMan integration",' + #13#10 +
      '  "path": "' + EscapedAppDir + '\\jadm-daemon.exe",' + #13#10 +
      '  "type": "stdio",' + #13#10 +
      '  "allowed_origins": [' + #13#10 +
      '    "chrome-extension://ipiefkjcicogeoepimgebinafoelhbhd/"' + #13#10 +
      '  ]' + #13#10 +
      '}';
      
    JsonFile := AppDir + '\com.jadm.jadm.json';
    SaveStringToFile(JsonFile, JsonContent, False);
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
  begin
    // Clean up PATH
    RemoveFromPath(ExpandConstant('{app}'));
    RemoveFromPath(ExpandConstant('{app}\bin'));
    SendBroadcastMessage($001A, 0, 'Environment');
  end;
end;
