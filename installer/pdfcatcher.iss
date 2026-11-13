; Installer PDF Catcher (Inno Setup 6)
; - Per pengguna, tanpa hak admin.
; - Folder instalasi bebas dipilih; default di drive lokal pertama selain C.
; - Memilih drive C dibolehkan setelah peringatan.
; - PC yang hanya punya drive C: default %LOCALAPPDATA%\Programs\PDF Catcher + pemberitahuan.
;
; Build: jalankan ..\build-installer.ps1 (build exe release dulu, lalu kompilasi skrip ini).

#define AppName "PDF Catcher"
#define AppVersion "1.1.0"
#define AppExe "pdfcatcher.exe"

[Setup]
AppId={{84CC496B-5962-4656-A65B-26FA2C946497}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=Darryl Fatha
DefaultDirName={code:GetDefaultDir}
DefaultGroupName={#AppName}
DisableDirPage=no
DisableProgramGroupPage=no
UsePreviousAppDir=yes
DirExistsWarning=no
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
OutputBaseFilename=PDFCatcher-Setup-{#AppVersion}
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
CloseApplications=yes
ShowLanguageDialog=no

[Languages]
Name: "id"; MessagesFile: "Indonesian.isl"

[CustomMessages]
id.FreeDirNote=Anda bebas memilih folder mana saja. Disarankan di luar drive C agar drive sistem tetap lega.
id.OnlyCNote=PC ini hanya punya drive C, jadi PDF Catcher dipasang di drive C. Anda tetap bisa memilih folder lain lewat tombol Telusuri.
id.DriveCWarn=Folder ini ada di drive C.%n%nDisarankan memasang PDF Catcher di drive lain agar drive sistem tetap lega. Tetap pasang di drive C?
id.NotWritable=Folder ini tidak bisa ditulisi tanpa hak administrator:%n%n%1%n%nSilakan pilih folder lain, misalnya di drive D atau di folder Dokumen Anda.
id.TaskShortcuts=Shortcut:
id.TaskDesktop=Buat shortcut di Desktop
id.TaskIntegration=Integrasi Windows:
id.TaskContext=Tambahkan "Buka dengan PDF Catcher" di menu klik kanan file PDF
id.ContextLabel=Buka dengan PDF Catcher
id.RunApp=Jalankan PDF Catcher
id.UninstallShortcut=Hapus PDF Catcher

[Tasks]
Name: "desktopicon"; Description: "{cm:TaskDesktop}"; GroupDescription: "{cm:TaskShortcuts}"
Name: "contextmenu"; Description: "{cm:TaskContext}"; GroupDescription: "{cm:TaskIntegration}"

[Files]
Source: "..\target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\vendor\pdfium.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\vendor\licenses\*"; DestDir: "{app}\licenses"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}\licenses"; DestName: "PDF-Catcher-LICENSE.txt"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{group}\{cm:UninstallShortcut}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Registry]
; Menu klik kanan untuk file PDF (di bawah HKCU, jadi tidak perlu admin).
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.pdf\shell\PDFCatcher"; ValueType: string; ValueName: ""; ValueData: "{cm:ContextLabel}"; Flags: uninsdeletekey; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.pdf\shell\PDFCatcher"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#AppExe}"",0"; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.pdf\shell\PDFCatcher\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" ""%1"""; Tasks: contextmenu

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:RunApp}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: files; Name: "{app}\config.json"
Type: dirifempty; Name: "{app}\licenses"
Type: dirifempty; Name: "{app}"

[Code]
const
  DRIVE_FIXED = 3;

var
  OnlyCDrive: Boolean;
  DefaultDir: String;
  NoteLabel: TNewStaticText;

function GetDriveType(lpRootPathName: String): Cardinal;
  external 'GetDriveTypeW@kernel32.dll stdcall';

{ Mencoba menulis lalu menghapus file uji di sebuah folder. }
function CanWriteDir(const Dir: String): Boolean;
var
  F: String;
begin
  F := AddBackslash(Dir) + 'pdfcatcher-uji-tulis.tmp';
  Result := SaveStringToFile(F, 'ok', False);
  if Result then
    DeleteFile(F);
end;

{ Drive lokal pertama selain C (D, E, ...) yang bisa ditulisi. }
function FindNonSystemDir(): String;
var
  C: Integer;
  Root: String;
begin
  Result := '';
  for C := Ord('D') to Ord('Z') do
  begin
    Root := Chr(C) + ':\';
    if (GetDriveType(Root) = DRIVE_FIXED) and DirExists(Root) and CanWriteDir(Root) then
    begin
      Result := Root + '{#AppName}';
      Exit;
    end;
  end;
end;

function InitializeSetup(): Boolean;
begin
  DefaultDir := FindNonSystemDir();
  OnlyCDrive := DefaultDir = '';
  if OnlyCDrive then
    DefaultDir := ExpandConstant('{localappdata}\Programs\{#AppName}');
  Result := True;
end;

function GetDefaultDir(Param: String): String;
begin
  Result := DefaultDir;
end;

procedure InitializeWizard();
begin
  NoteLabel := TNewStaticText.Create(WizardForm);
  NoteLabel.Parent := WizardForm.SelectDirPage;
  NoteLabel.AutoSize := False;
  NoteLabel.WordWrap := True;
  NoteLabel.Left := WizardForm.DirEdit.Left;
  NoteLabel.Top := WizardForm.DirEdit.Top + WizardForm.DirEdit.Height + ScaleY(14);
  NoteLabel.Width := WizardForm.SelectDirPage.Width - NoteLabel.Left;
  NoteLabel.Height := ScaleY(44);
  if OnlyCDrive then
  begin
    NoteLabel.Caption := CustomMessage('OnlyCNote');
    NoteLabel.Font.Style := [fsBold];
  end
  else
    NoteLabel.Caption := CustomMessage('FreeDirNote');
end;

function NextButtonClick(CurPageID: Integer): Boolean;
var
  Dir: String;
  Existed: Boolean;
begin
  Result := True;
  if CurPageID <> wpSelectDir then
    Exit;

  Dir := WizardDirValue;

  { Peringatan drive C (kecuali PC memang hanya punya drive C). }
  if (not OnlyCDrive) and (CompareText(Copy(Dir, 1, 2), 'C:') = 0) then
  begin
    if MsgBox(CustomMessage('DriveCWarn'), mbConfirmation, MB_YESNO) = IDNO then
    begin
      Result := False;
      Exit;
    end;
  end;

  { Pastikan folder bisa ditulisi tanpa admin. Folder yang dibuat untuk uji dihapus lagi. }
  Existed := DirExists(Dir);
  if (not ForceDirectories(Dir)) or (not CanWriteDir(Dir)) then
  begin
    if not Existed then
      RemoveDir(Dir);
    MsgBox(FmtMessage(CustomMessage('NotWritable'), [Dir]), mbError, MB_OK);
    Result := False;
    Exit;
  end;
  if not Existed then
    RemoveDir(Dir);
end;
