; Abakus Windows installer (Inno Setup 6).
; Build with packaging\build-installer.ps1, or manually:
;   iscc /DAppVersion=0.1.0 packaging\abakus.iss

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{73E010FC-F7F9-4927-8EB9-7BD06C13DC67}
AppName=Abakus
AppVersion={#AppVersion}
AppPublisher=SouthCarpet
PrivilegesRequired=lowest
DefaultDirName={localappdata}\Programs\Abakus
DefaultGroupName=Abakus
OutputBaseFilename=abakus-setup-{#AppVersion}
OutputDir=output
SetupIconFile=..\src-tauri\icons\icon.ico
UninstallDisplayIcon={app}\abakus.exe
CloseApplications=yes
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "slovak"; MessagesFile: "compiler:Languages\Slovak.isl"

[Tasks]
Name: "startmenuicon"; Description: "Vytvoriť odkaz v ponuke Štart"; GroupDescription: "Odkazy:"; Flags: checkedonce
Name: "desktopicon"; Description: "Vytvoriť odkaz na ploche"; GroupDescription: "Odkazy:"; Flags: checkedonce

[Files]
Source: "..\target\release\abakus.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\src-tauri\resources\pdfium\pdfium.dll"; DestDir: "{app}\resources\pdfium"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Abakus"; Filename: "{app}\abakus.exe"; Tasks: startmenuicon
Name: "{group}\Odinštalovať Abakus"; Filename: "{uninstallexe}"; Tasks: startmenuicon
Name: "{autodesktop}\Abakus"; Filename: "{app}\abakus.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\abakus.exe"; Description: "Spustiť Abakus"; Flags: postinstall nowait skipifsilent

[Code]
// Deletes every Windows Credential Manager entry the app wrote. secrets.rs uses
// keyring::Entry::new("abakus", iban), and the windows-native-keyring-store back
// end names each generic credential "<iban>.abakus" (empty prefix, "." delimiter,
// empty suffix are the crate defaults). cmdkey /list shows the target prefixed
// with "LegacyGeneric:target=" for a generic credential; cmdkey /delete takes the
// bare name without that prefix.
procedure DeleteAbakusCredentials();
var
  ResultCode: Integer;
  TempFile: String;
  Lines: TArrayOfString;
  I, MarkerPos: Integer;
  Line, Target: String;
begin
  TempFile := ExpandConstant('{tmp}\abakus-cmdkey-list.txt');
  Exec(ExpandConstant('{cmd}'), '/C cmdkey /list > "' + TempFile + '" 2>&1',
    '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  if LoadStringsFromFile(TempFile, Lines) then
  begin
    for I := 0 to GetArrayLength(Lines) - 1 do
    begin
      Line := Trim(Lines[I]);
      if Copy(Line, 1, 7) = 'Target:' then
      begin
        Target := Trim(Copy(Line, 8, MaxInt));
        MarkerPos := Pos('target=', Target);
        if MarkerPos > 0 then
          Target := Copy(Target, MarkerPos + 7, MaxInt);
        if (Length(Target) >= 7) and (Lowercase(Copy(Target, Length(Target) - 6, 7)) = '.abakus') then
          Exec(ExpandConstant('{cmd}'), '/C cmdkey /delete:"' + Target + '" >nul 2>&1',
            '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
      end;
    end;
  end;
  DeleteFile(TempFile);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  DeleteSettings, DeleteData: Integer;
begin
  if CurUninstallStep = usUninstall then
  begin
    if not UninstallSilent then
    begin
      DeleteSettings := MsgBox(
        'Vymazať používateľské nastavenia?' + #13#10 + #13#10 +
        'Vymaže iba heslá k výpisom uložené v Správcovi poverení systému Windows ' +
        '(položky Abakusu). Ostatné nastavenia appky sú v dátovom súbore, takže bez ' +
        'druhej otázky zostanú zachované.',
        mbConfirmation, MB_YESNO or MB_DEFBUTTON2);
      if DeleteSettings = IDYES then
        DeleteAbakusCredentials();

      DeleteData := MsgBox(
        'Vymazať používateľské údaje?' + #13#10 + #13#10 +
        'Vymaže priečinok ' + ExpandConstant('{localappdata}') + '\Abakus s databázou. ' +
        'Databáza obsahuje aj nastavenia uložené v appke a sieťový denník, takže táto ' +
        'voľba ich vymaže tiež.',
        mbConfirmation, MB_YESNO or MB_DEFBUTTON2);
      if DeleteData = IDYES then
        DelTree(ExpandConstant('{localappdata}\Abakus'), True, True, True);
    end;
  end;
end;
