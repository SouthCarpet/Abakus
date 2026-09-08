; Abakus Windows installer (Inno Setup 6).
; Build with packaging\build-installer.ps1, or manually:
;   iscc /DAppVersion=0.1.2 packaging\abakus.iss

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

; 0.1.4: /DWebView2Check=skip lets a CI-free or network-free build compile
; and run without the WebView2 registry check/prompt blocking it (documented
; in packaging\INSTALL.md). Any other value, including unset, runs the real
; check.
#ifndef WebView2Check
  #define WebView2Check "run"
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
Name: "startmenuicon"; Description: "Pridať odkaz do ponuky Štart"; GroupDescription: "Odkazy:"; Flags: checkedonce
Name: "desktopicon"; Description: "Pridať odkaz na plochu"; GroupDescription: "Odkazy:"; Flags: checkedonce

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
// 0.1.4 (Michal 2026-09-08): a machine without the WebView2 Evergreen
// runtime must still get a WORKING install of a Tauri app, not a silently
// broken one. `InitializeSetup` below checks for it before the wizard shows
// any page and, on the user's Yes, downloads and installs it.
const
  WebView2SubKeyWow = 'SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}';
  WebView2SubKeyNative = 'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}';
  WebView2BootstrapperUrl = 'https://go.microsoft.com/fwlink/p/?LinkId=2124703';

// Reads the Evergreen runtime's "pv" (product version) value from one
// registry hive/key. '' means the key or value is absent, the same signal
// an explicit "0.0.0.0" carries (a WebView2 client key can exist with that
// placeholder version before the runtime is actually installed).
function WebView2Version(const RootKey: Integer; const SubKeyName: String): String;
var
  Value: String;
begin
  if RegQueryStringValue(RootKey, SubKeyName, 'pv', Value) then
    Result := Value
  else
    Result := '';
end;

// Checked in the order most installs are actually found in: the
// 32-bit-on-64-bit key first (the vast majority of per-machine Evergreen
// installs), then the native key (ARM64 and some per-machine installs),
// then the per-user key (an unelevated Evergreen bootstrapper run, which is
// exactly what a decline-then-retry from this installer would produce,
// since PrivilegesRequired=lowest never elevates).
function WebView2Installed(): Boolean;
var
  Version: String;
begin
  Version := WebView2Version(HKLM, WebView2SubKeyWow);
  if (Version = '') or (Version = '0.0.0.0') then
    Version := WebView2Version(HKLM, WebView2SubKeyNative);
  if (Version = '') or (Version = '0.0.0.0') then
    Version := WebView2Version(HKCU, WebView2SubKeyNative);
  Result := (Version <> '') and (Version <> '0.0.0.0');
  if Result then
    Log('WebView2 check: found version ' + Version)
  else
    Log('WebView2 check: no usable "pv" value in any of the three registry keys');
end;

// {#WebView2Check} is an ISPP compile-time substitution (same pattern as
// {#AppVersion} above): '/DWebView2Check=skip' on the ISCC command line
// turns this into the literal text "skip"; unset, it is "run" (see the
// #ifndef block near the top of this file). Documented in
// packaging\INSTALL.md as a developer/CI-only escape hatch: a build using it
// never asks about or checks for WebView2 at all.
function WantsWebView2CheckSkipped(): Boolean;
begin
  Result := '{#WebView2Check}' = 'skip';
end;

// Runs before the wizard shows any page. Never requires admin: the
// Evergreen bootstrapper installs per-user when run unelevated, matching
// PrivilegesRequired=lowest above. A decline or a failure both continue the
// setup with a warning rather than stopping it: Michal's spec is "a working
// install", and refusing to install Abakus at all over a missing runtime
// the user can still add later would be worse than installing it and
// saying so. `SuppressibleMsgBox` (not `MsgBox`) throughout, so a
// /VERYSILENT run never blocks waiting for a click nobody will make; its
// `Default` result is what a silent run gets instead of asking.
function InitializeSetup(): Boolean;
var
  ResultCode: Integer;
  DownloadedBytes: Int64;
  BootstrapperPath: String;
  Choice: Integer;
begin
  Result := True;
  if WantsWebView2CheckSkipped() then
  begin
    Log('WebView2 check: skipped (/DWebView2Check=skip)');
    Exit;
  end;

  Log('WebView2 check: starting');
  if WebView2Installed() then
  begin
    Log('WebView2 check: runtime already present, nothing to do');
    Exit;
  end;

  Choice := SuppressibleMsgBox(
    'Abakus potrebuje Microsoft Edge WebView2 Runtime. Stiahnuť a nainštalovať teraz ' +
    '(asi 2 MB, vyžaduje internet)?',
    mbConfirmation, MB_YESNO, IDYES);
  if Choice = IDNO then
  begin
    Log('WebView2 check: user declined the download');
    SuppressibleMsgBox(
      'Pokračujem bez inštalácie WebView2 Runtime. Abakus sa nespustí, kým modul ' +
      'nebude nainštalovaný, aj samostatne neskôr.',
      mbInformation, MB_OK, IDOK);
    Exit;
  end;

  BootstrapperPath := ExpandConstant('{tmp}\MicrosoftEdgeWebview2Setup.exe');
  try
    DownloadedBytes := DownloadTemporaryFile(WebView2BootstrapperUrl, 'MicrosoftEdgeWebview2Setup.exe', '', nil);
    Log(Format('WebView2 check: bootstrapper downloaded, %d bytes', [DownloadedBytes]));
  except
    Log('WebView2 check: download failed: ' + GetExceptionMessage);
    SuppressibleMsgBox(
      'Stiahnutie modulu WebView2 Runtime zlyhalo: ' + GetExceptionMessage + '. Abakus ' +
      'sa nespustí, kým modul nebude nainštalovaný, aj samostatne neskôr.',
      mbError, MB_OK, IDOK);
    Exit;
  end;

  if not Exec(BootstrapperPath, '/silent /install', '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then
  begin
    Log('WebView2 check: bootstrapper failed to start');
    SuppressibleMsgBox(
      'Inštalátor modulu WebView2 Runtime sa nepodarilo spustiť. Abakus sa nespustí, ' +
      'kým modul nebude nainštalovaný, aj samostatne neskôr.',
      mbError, MB_OK, IDOK);
    Exit;
  end;

  Log(Format('WebView2 check: bootstrapper finished, exit code %d', [ResultCode]));
  if WebView2Installed() then
    Log('WebView2 check: runtime detected after install')
  else
  begin
    Log('WebView2 check: runtime still not detected after the bootstrapper ran');
    SuppressibleMsgBox(
      'Modul WebView2 Runtime sa aj po inštalácii nepodarilo overiť. Abakus sa nemusí spustiť.',
      mbError, MB_OK, IDOK);
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
