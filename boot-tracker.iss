; -*- coding: utf-8 -*-
; Inno Setup 安装脚本 — 开机记录 (BootTracker)
;
; 编译方法：
;   1. 安装 Inno Setup 6（https://jrsoftware.org/isdl.php）
;   2. 用 Inno Setup Compiler 打开本文件 boot-tracker.iss
;   3. 点击 Build → Compile（或 Ctrl+F9）
;   4. 生成的安装程序位于 installer_output\BootTracker-Setup-1.0.0.exe
;
; 前置条件：已通过 PyInstaller 生成 dist\BootTracker\ 目录
;   命令： pyinstaller boot-tracker.spec --noconfirm

[Setup]
AppName=开机记录
AppVersion=1.0.0
AppPublisher=BootTracker
AppPublisherURL=https://github.com/boottracker
AppSupportURL=https://github.com/boottracker
DefaultDirName={localappdata}\Programs\BootTracker
DefaultGroupName=开机记录
; 用户级安装（无需管理员权限），装到 LocalAppData 保证数据可写
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
Compression=lzma2
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64
ArchitecturesAllowed=x64
DisableProgramGroupPage=yes
DisableDirPage=no
WizardStyle=modern
OutputDir=installer_output
OutputBaseFilename=BootTracker-Setup-1.0.0
SetupIconFile=icon.ico
UninstallDisplayIcon={app}\开机记录.exe
UninstallDisplayName=开机记录
; 安装时若应用正在运行，提示关闭
CloseApplications=force

[Languages]
Name: "chinesesimp"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "创建桌面快捷方式"; GroupDescription: "附加任务:"
Name: "autostart"; Description: "开机自动启动"; GroupDescription: "附加任务:"

[Files]
; 打包生成的整个应用目录
Source: "dist\BootTracker\*"; DestDir: "{app}"; Flags: recursesubdirs ignoreversion createallsubdirs

[Icons]
; 开始菜单快捷方式
Name: "{group}\开机记录"; Filename: "{app}\开机记录.exe"; IconFilename: "{app}\开机记录.exe"
Name: "{group}\卸载开机记录"; Filename: "{uninstallexe}"
; 桌面快捷方式（可选）
Name: "{commondesktop}\开机记录"; Filename: "{app}\开机记录.exe"; IconFilename: "{app}\开机记录.exe"; Tasks: desktopicon
; 开机自启（可选，通过启动文件夹快捷方式）
Name: "{autostartup}\开机记录"; Filename: "{app}\开机记录.exe"; IconFilename: "{app}\开机记录.exe"; Tasks: autostart

[Run]
; 安装完成后可选择立即启动
Filename: "{app}\开机记录.exe"; Description: "立即启动开机记录"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; 卸载前尝试关闭应用
Filename: "{cmd}"; Parameters: "/C taskkill /IM ""开机记录.exe"" /F"; Flags: runhidden; RunOnceId: "KillApp"

[UninstallDelete]
; 卸载时清理备份目录（用户数据 boot-data.json 等保留，由用户自行处理）
Type: filesandordirs; Name: "{app}\backup"
Type: filesandordirs; Name: "{app}\static\uploads"
