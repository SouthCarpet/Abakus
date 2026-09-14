; Abakus Windows installer (Inno Setup 6).
; Build with packaging\build-installer.ps1 (includes payload validation).

#ifndef ExeSha256
  #error Build with packaging\build-installer.ps1: missing validated EXE identity
#endif
#ifndef PdfiumSha256
  #error Build with packaging\build-installer.ps1: missing validated PDFium pin
#endif
#if GetSHA256OfFile('..\target\release\abakus.exe') != ExeSha256
  #error EXE changed after payload validation
#endif
#if GetSHA256OfFile('..\src-tauri\resources\pdfium\pdfium.dll') != PdfiumSha256
  #error PDFium changed after payload validation
#endif

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

; Developer-only test build: skip registry/download work, keep launch disabled.
#ifndef WebView2Check
  #define WebView2Check "run"
#endif

; 0.1.5: /DAppIdGuid=<guid> on the ISCC command line swaps in a throwaway
; AppId for acceptance testing (packaging\INSTALL.md, developer-only), so a
; test run of the update/reinstall/downgrade check never touches the real
; Abakus install or its uninstall registry key. Unset (the default release
; build) keeps the real AppId below, unchanged from every earlier release.
#ifndef AppIdGuid
  #define AppIdGuid "73E010FC-F7F9-4927-8EB9-7BD06C13DC67"
#endif

[Setup]
AppId={{{#AppIdGuid}}
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
MinVersion=10.0

[Languages]
Name: "slovak"; MessagesFile: "compiler:Languages\Slovak.isl"

[Tasks]
Name: "startmenuicon"; Description: "Pridať odkaz do ponuky Štart"; GroupDescription: "Odkazy:"; Flags: checkedonce
Name: "desktopicon"; Description: "Pridať odkaz na plochu"; GroupDescription: "Odkazy:"; Flags: checkedonce

[Files]
Source: "..\target\release\abakus.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\src-tauri\resources\pdfium\pdfium.dll"; DestDir: "{app}\resources\pdfium"; Flags: ignoreversion; Check: PdfiumNeedsCopy
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Abakus"; Filename: "{app}\abakus.exe"; Tasks: startmenuicon
Name: "{group}\Odinštalovať Abakus"; Filename: "{uninstallexe}"; Tasks: startmenuicon
Name: "{autodesktop}\Abakus"; Filename: "{app}\abakus.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\abakus.exe"; Description: "Spustiť Abakus"; Flags: postinstall nowait skipifsilent; Check: CanLaunch

[Code]
const
  WebView2SubKeyNative = 'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}';
  WebView2BootstrapperUrl = 'https://go.microsoft.com/fwlink/p/?LinkId=2124703';

var
  DependenciesReady: Boolean;
  PayloadFailed: Boolean;

function NumericVersionShape(const Value: String): Boolean;
var
  I, Dots: Integer;
begin
  Result := False;
  if (Value = '') or (Pos('..', Value) > 0) then Exit;
  if (Value[1] = '.') or (Value[Length(Value)] = '.') then Exit;
  Dots := 0;
  for I := 1 to Length(Value) do
    if Value[I] = '.' then
      Dots := Dots + 1
    else if (Value[I] < '0') or (Value[I] > '9') then Exit;
  Result := Dots = 3;
end;

function RegisteredWebView2(const RootKey: Integer): Boolean;
var
  Version: String;
  Packed: Int64;
begin
  Result := False;
  if not RegQueryStringValue(RootKey, WebView2SubKeyNative, 'pv', Version) then Exit;
  if not NumericVersionShape(Version) then Exit;
  if not StrToVersion(Version, Packed) then Exit;
  Result := ComparePackedVersion(Packed, 0) > 0;
  if Result then
    Log('WebView2: registered version ' + Version + '; runtime health not tested');
end;

function WebView2Installed(): Boolean;
begin
  // Explicit views avoid combining WOW6432Node with an implicit view.
  Result := RegisteredWebView2(HKLM32) or RegisteredWebView2(HKLM64) or
    RegisteredWebView2(HKCU32) or RegisteredWebView2(HKCU64);
end;

// {#WebView2Check} is an ISPP compile-time substitution (same pattern as
// {#AppVersion} above): '/DWebView2Check=skip' on the ISCC command line
// turns this into the literal text "skip"; unset, it is "run" (see the
// #ifndef block near the top of this file). Documented in
// packaging\INSTALL.md as a developer-only test build with launch disabled.
function WantsWebView2CheckSkipped(): Boolean;
begin
  Result := '{#WebView2Check}' = 'skip';
end;

// 0.1.5 (Michal 2026-09-09): a second run of this installer over an
// existing install must say so, instead of silently updating, reinstalling
// or downgrading without a word. The fixed AppId means Inno already
// updates the same {AppId}_is1 registry key and files in place; this reads
// what is already there so the wizard can name it.
const
  UninstallKeyName = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{{#AppIdGuid}}_is1';

var
  ExistingInstallFound: Boolean;
  ExistingInstallVersionStr: String;
  ExistingInstallDir: String;

// DisplayVersion and InstallLocation are both written automatically by
// Inno Setup's own uninstall registration (no [Code] on our side writes
// them). False means no previous install was found under this AppId.
function ExistingInstallVersion(var OldVersion, OldDir: String): Boolean;
begin
  Result := RegQueryStringValue(HKCU, UninstallKeyName, 'DisplayVersion', OldVersion) and
    RegQueryStringValue(HKCU, UninstallKeyName, 'InstallLocation', OldDir);
  if Result then
    Log('Existing install check: found DisplayVersion=' + OldVersion + ', InstallLocation=' + OldDir)
  else
    Log('Existing install check: no existing install found under HKCU\' + UninstallKeyName);
end;

procedure DependencyWarning(const Detail: String);
begin
  Log('Dependencies: deferred; ' + Detail);
  if not WizardSilent then
    SuppressibleMsgBox(Detail + #13#10 +
      'Ponuka spustenia Abakusu bude vypnutá. ' +
      'Odstráňte uvedený problém a zopakujte inštaláciu Abakusu.',
      mbError, MB_OK, IDOK);
end;

function CheckWebView2(): Boolean;
var
  ResultCode: Integer;
  DownloadedBytes: Int64;
  BootstrapperPath: String;
begin
  Result := False;
  if WantsWebView2CheckSkipped() then
  begin
    Log('WebView2: check skipped in test build; launch disabled');
    Exit;
  end;
  Result := WebView2Installed();
  if Result then Exit;
  if WizardSilent then
  begin
    Log('Dependencies: missing WebView2; silent setup aborted before file copy; no download');
    Exit;
  end;
  if SuppressibleMsgBox(
    'Abakus potrebuje Microsoft Edge WebView2 Runtime. Stiahnuť a nainštalovať teraz? ' +
    'Vyžaduje internet. Malý inštalátor stiahne aj samotný modul.',
    mbConfirmation, MB_YESNO or MB_DEFBUTTON2, IDNO) <> IDYES then
  begin
    DependencyWarning('Inštalácia WebView2 Runtime bola odložená.');
    Exit;
  end;
  BootstrapperPath := ExpandConstant('{tmp}\MicrosoftEdgeWebview2Setup.exe');
  try
    DownloadedBytes := DownloadTemporaryFile(WebView2BootstrapperUrl, 'MicrosoftEdgeWebview2Setup.exe', '', nil);
    Log(Format('WebView2: bootstrapper downloaded, %d bytes', [DownloadedBytes]));
  except
    DependencyWarning('Stiahnutie WebView2 Runtime zlyhalo: ' + GetExceptionMessage);
    Exit;
  end;
  if not Exec(BootstrapperPath, '/silent /install', '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then
  begin
    DependencyWarning('Inštalátor WebView2 Runtime sa nepodarilo spustiť.');
    Exit;
  end;
  Log(Format('WebView2: bootstrapper exit code %d', [ResultCode]));
  if ResultCode <> 0 then
  begin
    DependencyWarning('Inštalátor WebView2 Runtime ohlásil chybu. Kód: ' + IntToStr(ResultCode));
    Exit;
  end;
  Result := WebView2Installed();
  if not Result then
    DependencyWarning('WebView2 Runtime nemá platný záznam ani po inštalácii.');
end;

// Silent update/reinstall proceed; silent downgrade is refused.
// The interactive defaults are unchanged.
function InstallChoice(const Text: String; Kind: TMsgBoxType; Buttons, Default: Integer): Integer;
begin
  if WizardSilent then Result := Default
  else Result := SuppressibleMsgBox(Text, Kind, Buttons, Default);
end;
function ConfirmExistingInstall(): Boolean;
var
  Choice: Integer;
  OldVer, NewVer: Int64;
  Cmp: Integer;
begin
  Result := True;

  ExistingInstallFound := ExistingInstallVersion(ExistingInstallVersionStr, ExistingInstallDir);
  if not ExistingInstallFound then
    Exit;

  if not StrToVersion(ExistingInstallVersionStr, OldVer) then
  begin
    Log('Existing install check: could not parse existing DisplayVersion "' +
      ExistingInstallVersionStr + '" as a version number, skipping the update/reinstall/downgrade check');
    Exit;
  end;
  if not StrToVersion('{#AppVersion}', NewVer) then
  begin
    Log('Existing install check: could not parse installer AppVersion "{#AppVersion}" as a version number, ' +
      'skipping the update/reinstall/downgrade check');
    Exit;
  end;

  Cmp := ComparePackedVersion(OldVer, NewVer);
  if Cmp < 0 then
  begin
    Log('Existing install check: older install ' + ExistingInstallVersionStr + ' at ' +
      ExistingInstallDir + ' -> update to {#AppVersion}');
    Choice := InstallChoice(
      'Abakus ' + ExistingInstallVersionStr + ' je už nainštalovaný v ' + ExistingInstallDir + '. ' +
      'Inštalátor ho aktualizuje na {#AppVersion}. Údaje v %LOCALAPPDATA%\Abakus a heslá v ' +
      'Správcovi poverení zostanú.',
      mbInformation, MB_OKCANCEL, IDOK);
    if Choice = IDCANCEL then
    begin
      Log('Existing install check: user cancelled the update');
      Result := False;
    end;
  end
  else if Cmp = 0 then
  begin
    Log('Existing install check: same version ' + ExistingInstallVersionStr + ' at ' +
      ExistingInstallDir + ' -> asking to reinstall');
    Choice := InstallChoice(
      'Abakus ' + ExistingInstallVersionStr + ' je už nainštalovaný v ' + ExistingInstallDir + '. ' +
      'Chcete ho preinštalovať?',
      mbConfirmation, MB_YESNO, IDYES);
    if Choice = IDNO then
    begin
      Log('Existing install check: user declined the reinstall');
      Result := False;
    end;
  end
  else
  begin
    Log('Existing install check: installed version ' + ExistingInstallVersionStr + ' at ' +
      ExistingInstallDir + ' is newer than installer {#AppVersion}');
    Choice := InstallChoice(
      'Nainštalovaná verzia ' + ExistingInstallVersionStr + ' je novšia ako {#AppVersion}. ' +
      'Chcete ju nahradiť staršou verziou?',
      mbConfirmation, MB_YESNO, IDNO);
    if Choice = IDNO then
    begin
      Log('Existing install check: user kept the newer installed version, cancelling setup');
      Result := False;
    end;
  end;
end;

function InitializeSetup(): Boolean;
begin
  Result := ConfirmExistingInstall();
  if not Result then Exit;
  Log('Dependencies: Windows 10+; x64-compatible platform accepted by Setup');
  DependenciesReady := CheckWebView2();
  Result := DependenciesReady or (not WizardSilent) or WantsWebView2CheckSkipped();
end;

function FileMatches(const Path, Expected: String): Boolean;
begin
  Result := False;
  if not FileExists(Path) then Exit;
  try
    Result := CompareText(GetSHA256OfFile(Path), Expected) = 0;
  except
    Log('Dependencies: cannot read ' + Path + ': ' + GetExceptionMessage);
  end;
end;

function PdfiumNeedsCopy(): Boolean;
begin
  Result := not FileMatches(ExpandConstant('{app}\resources\pdfium\pdfium.dll'), '{#PdfiumSha256}');
end;

function CanLaunch(): Boolean;
begin
  Result := DependenciesReady;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep <> ssPostInstall then Exit;
  PayloadFailed := not (FileMatches(ExpandConstant('{app}\resources\pdfium\pdfium.dll'), '{#PdfiumSha256}') and
    FileMatches(ExpandConstant('{app}\abakus.exe'), '{#ExeSha256}'));
  DependenciesReady := DependenciesReady and (not PayloadFailed);
  if PayloadFailed then
    DependencyWarning('Kontrola nainštalovaných súborov zlyhala (abakus.exe alebo pdfium.dll). ' +
      'Zopakujte inštaláciu z dôveryhodného inštalátora.');
  if DependenciesReady then
    Log('Dependencies: payload hashes match and WebView2 is registered; runtime health not tested')
  else
    Log('Dependencies: incomplete; app launch disabled');
end;

function GetCustomSetupExitCode(): Integer;
begin
  Result := 0;
  if PayloadFailed then Result := 4;
end;

// The tasks page (startmenuicon/desktopicon) is the redundant page from
// Michal's report: Inno's own UsePreviousTasks already keeps the previous
// choice on an update or reinstall, so asking again shows a page whose
// answer is thrown away. Fresh install: not skipped, unchanged.
function ShouldSkipPage(PageID: Integer): Boolean;
begin
  Result := ExistingInstallFound and (PageID = wpSelectTasks);
  if Result then
    Log('ShouldSkipPage: skipping wpSelectTasks (existing install found, previous task choices are kept)');
end;

// Names what this run will do. NOT wpWelcome: on an update or reinstall,
// InitializeSetup's own message box already ends Setup on Cancel/No before
// the wizard shows any page, and Inno never shows wpWelcome again after
// that message box, it goes straight to wpReady, so a caption written to
// WelcomeLabel2 here would be dead code (found by independent review with
// a UI run and a screenshot, 2026-09-09). wpReady is the page that always
// shows next, both for a fresh install and for an update/reinstall, so
// that is where this text belongs. Fresh install: not touched, wording
// unchanged.
procedure CurPageChanged(CurPageID: Integer);
begin
  if (CurPageID = wpFinished) and (not DependenciesReady) then
    WizardForm.FinishedLabel.Caption :=
      'Inštalácia vyžaduje opravu alebo doplnenie závislostí. Abakus sa teraz nespustí. ' +
      'Pozrite si uvedené upozornenie a inštalačný denník.';
  if (CurPageID = wpReady) and ExistingInstallFound then
  begin
    WizardForm.ReadyLabel.Caption :=
      'Aktualizácia Abakus ' + ExistingInstallVersionStr + ' na {#AppVersion}.' + #13#10 +
      ExistingInstallDir;
    Log('CurPageChanged: rewrote ReadyLabel caption for update/reinstall (' +
      ExistingInstallVersionStr + ' -> {#AppVersion})');
  end;
  if CurPageID = wpReady then
  begin
    WizardForm.ReadyLabel.Caption := WizardForm.ReadyLabel.Caption + #13#10 +
      'Platforma: Windows 10 alebo novší, x64 kompatibilný. PDFium: overený súbor v balíku.';
    if DependenciesReady then
      WizardForm.ReadyLabel.Caption := WizardForm.ReadyLabel.Caption + #13#10 +
        'WebView2: platný záznam verzie. Funkčnosť modulu sa tým netestuje.'
    else
      WizardForm.ReadyLabel.Caption := WizardForm.ReadyLabel.Caption + #13#10 +
        'WebView2: chýba alebo kontrola bola preskočená. Spustenie bude vypnuté.';
  end;
end;

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
        'Odstránia sa len heslá k výpisom uložené v Správcovi poverení systému Windows ' +
        '(položky Abakusu). Ostatné nastavenia zostanú zachované.',
        mbConfirmation, MB_YESNO or MB_DEFBUTTON2);
      if DeleteSettings = IDYES then
        DeleteAbakusCredentials();

      DeleteData := MsgBox(
        'Vymazať používateľské údaje?' + #13#10 + #13#10 +
        'Odstráni sa priečinok ' + ExpandConstant('{localappdata}') + '\Abakus s databázou, ' +
        'nastaveniami a sieťovým denníkom. Túto operáciu nemožno vrátiť späť.',
        mbConfirmation, MB_YESNO or MB_DEFBUTTON2);
      if DeleteData = IDYES then
        DelTree(ExpandConstant('{localappdata}\Abakus'), True, True, True);
    end;
  end;
end;
