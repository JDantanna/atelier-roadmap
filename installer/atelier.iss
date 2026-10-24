#ifndef AppVersion
  #define AppVersion "0.2.0"
#endif
[Setup]
AppId=AtelierRoadmap
AppName=Atelier Roadmap
AppVersion={#AppVersion}
AppPublisher=JDantanna
AppPublisherURL=https://github.com/JDantanna/atelier-roadmap
DefaultDirName={localappdata}\Programs\Atelier
DefaultGroupName=Atelier
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir=..\dist
OutputBaseFilename=Atelier-Setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\Atelier.exe
CloseApplications=yes
RestartApplications=no
[Languages]
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
[Tasks]
Name: "desktopicon"; Description: "Criar atalho na área de trabalho"; Flags: unchecked
[Files]
Source: "..\target\release\atelier-roadmap.exe"; DestDir: "{app}"; DestName: "Atelier.exe"; Flags: ignoreversion
Source: "..\dist\MicrosoftEdgeWebview2Setup.exe"; Flags: dontcopy
[Icons]
Name: "{group}\Atelier"; Filename: "{app}\Atelier.exe"
Name: "{autodesktop}\Atelier"; Filename: "{app}\Atelier.exe"; Tasks: desktopicon
[Run]
Filename: "{app}\Atelier.exe"; Flags: nowait
[Code]
function HasWebView2: Boolean;
var Version: String;
begin
  Result := (RegQueryStringValue(HKLM32, 'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', Version) or
    RegQueryStringValue(HKCU, 'Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', Version)) and (Version <> '') and (Version <> '0.0.0.0');
end;
function PrepareToInstall(var NeedsRestart: Boolean): String;
var Code: Integer;
begin
  Result := '';
  if not HasWebView2 then begin
    ExtractTemporaryFile('MicrosoftEdgeWebview2Setup.exe');
    if not Exec(ExpandConstant('{tmp}\MicrosoftEdgeWebview2Setup.exe'), '/silent /install', '', SW_HIDE, ewWaitUntilTerminated, Code) then
      Result := 'Não foi possível preparar o Microsoft WebView2. Verifique sua conexão e execute o instalador novamente.'
    else if not HasWebView2 then
      Result := 'O Microsoft WebView2 ainda não está disponível. Conecte à internet e tente novamente.';
  end;
end;
