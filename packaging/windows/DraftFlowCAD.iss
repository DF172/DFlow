; ============================================================================
;  DraftFlow CAD —— Windows 安装脚本（Inno Setup 7.1 或更高版本）
;
;  编译：  D:\app\innosetup-7.1.1-x64\ISCC.exe packaging\windows\DraftFlowCAD.iss
;  产物：  source\dist\DraftFlowCAD-1.0.0-x64-setup.exe
;
;  说明：
;   * 本脚本使用 Inno Setup 7 的原生深色模式（WizardStyle=modern dark）。
;     安装向导与卸载器的按钮、进度条、单选框、复选框、滚动条、输入框、
;     列表、消息框全部由内置深色 VCL 样式绘制，脚本不手改任何控件颜色。
;   * AppId 一经发布永不修改，否则覆盖升级会被当成另一款产品。
;   * 本文件编码为 UTF-8 with BOM，请勿另存为 ANSI。
;
;  ── 品牌定制（只靠 4 条 [Setup] 指令，没有一行重绘代码）──────────────────
;     WizardStyle=modern dark hidebevels includetitlebar
;         modern     页面版式；深色样式激活时脚本里设置的颜色属性会被样式
;                    覆盖，所以不要再写 StyleLabel / 颜色常量 / DWM API。
;         dark       强制深色，不跟随系统浅色设置。
;         hidebevels 去掉所有横向分隔浅线。
;         includetitlebar  标题栏与窗口边框也走深色样式（不再调 DwmSetWindowAttribute）。
;     WizardBackColor=#141619            页面与所有对话框底色（应用同款深灰）
;     WizardImageBackColor=#141619       左侧大图未覆盖处的填边色 → 与右栏无缝
;     WizardSmallImageBackColor=#141619  右上角小图填边色 → 与页头无缝
;
;  ── 品牌图片尺寸（WizardImageFile / WizardSmallImageFile）──────────────────
;   Inno 6.6 起向导尺寸变大，7.1 的图片区域：
;   左侧大图（比例 164:314，向下兼容旧图，但会拉伸）：
;       100% DPI  202 × 386 px   ← WizardLarge.bmp
;       200% DPI  430 × 824 px   ← WizardLarge@2x.bmp
;   右上角小图（正方形）：
;       100% DPI   58 ×  58 px   ← WizardSmall.bmp
;       200% DPI  124 × 124 px   ← WizardSmall@2x.bmp
;   图必须是 #141619 深底，才能和 WizardImageBackColor 填边色融为一体。
; ============================================================================

; ── 产品基本信息 ────────────────────────────────────────────────────────────
#define MyAppName "DraftFlow CAD"
#define MyAppVersion "1.0.1"          ; ← 与 Cargo.toml 的 version、exe 的 FileVersion 同步
#define MyAppVersionInfo MyAppVersion + ".0"  ; VersionInfo 需要 4 段；版本改为 4 段时删掉拼接
#define MyAppPublisher "DraftFlow CAD"
#define MyAppURL "https://github.com/DF172/DFlow"
#define MyAppExeName "DraftFlowCAD.exe"
#define MyShortcutName "流绘 CAD"     ; 开始菜单与桌面快捷方式的显示名（界面语言为中文）

; AppId 与 MSI 的 UpgradeCode 同源，仅表示“同一款产品”；
; MSI 与 Inno 之间不会自动互相升级，正式发布时二选一。
#define MyAppId "{{BD6BCEAB-6C7A-4CE2-A62D-6EF06A92F8B1}"

; ★ 图标占位符：换成你自己的 .ico（相对本 .iss 或绝对路径均可）。
;   安装后必须仍叫 dwg.ico / dxf.ico —— 应用运行时会按这两个固定文件名在 exe 同目录查找。
#define DwgIcon "dwg.ico"
#define DxfIcon "dxf.ico"

; ── 品牌图片探测 ────────────────────────────────────────────────────────────
; 100% 基准图缺失时直接编译失败，避免装出“一半暗色一半默认浅色”的向导。
#if !FileExists(AddBackslash(SourcePath) + "WizardLarge.bmp")
  #error 缺少 packaging\windows\WizardLarge.bmp（左侧大图，100% 尺寸 202x386）
#endif
#if !FileExists(AddBackslash(SourcePath) + "WizardSmall.bmp")
  #error 缺少 packaging\windows\WizardSmall.bmp（右上小图，100% 尺寸 58x58）
#endif

; 200% 图存在就一起打包进安装器，不存在则只用 100% 图（会被拉伸）。
#if FileExists(AddBackslash(SourcePath) + "WizardLarge@2x.bmp")
  #define WizardLargeFiles "WizardLarge.bmp,WizardLarge@2x.bmp"
#else
  #define WizardLargeFiles "WizardLarge.bmp"
#endif
#if FileExists(AddBackslash(SourcePath) + "WizardSmall@2x.bmp")
  #define WizardSmallFiles "WizardSmall.bmp,WizardSmall@2x.bmp"
#else
  #define WizardSmallFiles "WizardSmall.bmp"
#endif

; ── 载荷路径探测 ────────────────────────────────────────────────────────────
; 本仓库 .cargo/config.toml 把 target 重定向到了 D:\cargo-target；换机器时改这一行。
#define CargoTargetDir "D:\cargo-target"

#define MyAppExe1 AddBackslash(CargoTargetDir) + "x86_64-pc-windows-msvc\release\DraftFlowCAD.exe"
#define MyAppExe5 AddBackslash(CargoTargetDir) + "release\DraftFlowCAD.exe"   ; CARGO_TARGET_DIR 未加 target 三元组时的产物
#define MyAppExe2 AddBackslash(SourcePath) + "..\..\target\x86_64-pc-windows-msvc\release\DraftFlowCAD.exe"
#define MyAppExe3 AddBackslash(SourcePath) + "..\..\target\release\DraftFlowCAD.exe"
#define MyAppExe4 AddBackslash(SourcePath) + "..\..\..\DraftFlowCAD.exe"

#if FileExists(MyAppExe1)
  #define MyAppExe MyAppExe1
#elif FileExists(MyAppExe5)
  #define MyAppExe MyAppExe5
#elif FileExists(MyAppExe2)
  #define MyAppExe MyAppExe2
#elif FileExists(MyAppExe3)
  #define MyAppExe MyAppExe3
#elif FileExists(MyAppExe4)
  #define MyAppExe MyAppExe4
#else
  #error DraftFlowCAD.exe not found. Run "cargo build --release" first, or edit MyAppExe1..4.
#endif

; ============================================================================
[Setup]
; ── 身份与版本 ──────────────────────────────────────────────────────────────
AppId={#MyAppId}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
AppCopyright=Copyright (C) 2026 DraftFlow CAD. GNU GPL v3.
AppComments=2D/3D CAD application for DWG and DXF drawings.
VersionInfoVersion={#MyAppVersionInfo}
VersionInfoProductVersion={#MyAppVersionInfo}
VersionInfoCompany={#MyAppPublisher}
VersionInfoProductName={#MyAppName}
VersionInfoDescription={#MyAppName} Setup
UninstallDisplayName={#MyAppName} {#MyAppVersion}
UninstallDisplayIcon={app}\{#MyAppExeName}

; ── 安装范围：默认“仅为我安装”，安装程序会弹出选择框 ───────────────────────
; 选“为所有用户安装”→ {autopf}=Program Files、注册表写 HKLM（会请求 UAC）。
; 选“仅为我安装”    → {autopf}={localappdata}\Programs、注册表写 HKCU（不请求 UAC）。
; 静默部署可用 /ALLUSERS 或 /CURRENTUSER 强制指定。
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog commandline

; ── 目录与平台 ──────────────────────────────────────────────────────────────
DefaultDirName={autopf}\DraftFlow CAD
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
DisableDirPage=auto
AlwaysShowDirOnReadyPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0

; ── 升级行为：同一个 AppId + 记住上次目录/任务 = 原地覆盖，不会装出第二份 ──
UsePreviousAppDir=yes
UsePreviousTasks=yes

; ── 外观与体验：Inno Setup 7 原生深色 ───────────────────────────────────────
; dark = 强制深色，由内置深色 VCL 样式绘制全部控件（按钮、进度条、单选/
; 复选框、滚动条、输入框、列表、消息框），安装器与卸载器都生效。
; 自定义样式激活时，Pascal 里设置的颜色属性会被样式覆盖 —— 所以 [Code]
; 里不要再做任何上色（本脚本已无一行重绘代码）。
WizardStyle=modern dark hidebevels includetitlebar
; 页面底色 = DraftFlow CAD 应用深灰；设置后所有面板变透明，无白斑
WizardBackColor=#141619
; 大图/小图未覆盖处的填边色，与页面同色 → 图片与背景无缝
WizardImageBackColor=#141619
WizardSmallImageBackColor=#141619
; 必须显式关掉 auto：只要指定了 LicenseFile，Inno 默认会把欢迎页自动隐藏，
; 那样“即将安装 DraftFlow CAD”的文案永远不会出现。
DisableWelcomePage=no
; 品牌大图 / 小图（多 DPI 用逗号分隔；相对本脚本目录）
WizardImageFile={#WizardLargeFiles}
WizardSmallImageFile={#WizardSmallFiles}
; 图片区域大于图片时，用上面的填边色补齐
WizardImageStretch=yes
SetupIconFile=AppIcon.ico
; 许可协议：中文导读（GPLv3 权利/义务/免责要点）+ 英文原文全文，RTF 排版。
; GNU 未发布 GPLv3 正式中文译本，故页首明确“中文仅供阅读参考、以英文原文为准”。
; 深色样式会清除 RTF 自带颜色，只保留字体/字号/粗体，正好由向导统一下色。
LicenseFile=License.zh-CN.rtf
SetupLogging=yes

; 应用没有命名互斥体（单实例用的是 TCP listener），所以不写 AppMutex；
; 升级时靠 Restart Manager 关闭正在运行的 DraftFlowCAD.exe。
CloseApplications=yes
RestartApplications=no
SetupMutex=DraftFlowCADSetup

; 写入/删除文件关联后通知 Explorer 刷新
ChangesAssociations=yes

; ── 压缩与产物 ──────────────────────────────────────────────────────────────
Compression=lzma2/max
SolidCompression=yes
OutputDir=..\..\dist
OutputBaseFilename=DraftFlowCAD-{#MyAppVersion}-x64-setup

; ── 数字签名（可选，配好签名工具后取消注释）────────────────────────────────
; SignTool=signtool
; SignedUninstaller=yes

; ============================================================================
;  界面语言：Inno 官方不带简体中文语言文件，这里直接覆盖本向导会出现的消息。
;  未覆盖的条目仍回退英文；升级 Inno 后如需补全，对照 Default.isl 的键名添行即可。
[Messages]
; ── 窗口与通用 ──────────────────────────────────────────────────────────────
SetupAppTitle=安装程序
SetupWindowTitle=安装程序 - %1
UninstallAppTitle=卸载
UninstallAppFullTitle=%1 卸载
InformationTitle=提示
ConfirmTitle=确认
ErrorTitle=错误
ExitSetupTitle=退出安装
ExitSetupMessage=安装尚未完成。如果现在退出，程序将不会被安装。%n%n你可以稍后再次运行安装程序完成安装。%n%n要退出安装吗？
ClickNext=单击“下一步”继续，或单击“取消”退出安装。

; ── 按钮 ────────────────────────────────────────────────────────────────────
ButtonBack=< 上一步(&B)
ButtonNext=下一步(&N) >
ButtonInstall=安装(&I)
ButtonCancel=取消
ButtonOK=确定
ButtonYes=是(&Y)
ButtonNo=否(&N)
ButtonFinish=完成(&F)
ButtonBrowse=浏览(&B)...
ButtonWizardBrowse=浏览(&R)...

; ── 安装模式选择页 ──────────────────────────────────────────────────────────
PrivilegesRequiredOverrideTitle=选择安装模式
PrivilegesRequiredOverrideInstruction=选择安装方式
PrivilegesRequiredOverrideText1=%1 可以为这台电脑上的所有用户安装（需要管理员权限），也可以只为你自己安装。
PrivilegesRequiredOverrideText2=%1 可以只为你自己安装，也可以为这台电脑上的所有用户安装（需要管理员权限）。
PrivilegesRequiredOverrideAllUsers=为所有用户安装(&A)
PrivilegesRequiredOverrideAllUsersRecommended=为所有用户安装(&A)（推荐）
PrivilegesRequiredOverrideCurrentUser=仅为我安装(&M)
PrivilegesRequiredOverrideCurrentUserRecommended=仅为我安装(&M)（推荐）

; ── 欢迎页：标题 + 副标题 ───────────────────────────────────────────────────
WelcomeLabel1=即将安装 DraftFlow CAD
WelcomeLabel2=属于年轻人的下一代新式 CAD。

; ── 许可协议页 ──────────────────────────────────────────────────────────────
WizardLicense=许可协议
LicenseLabel=请先阅读以下重要信息，再继续。
LicenseLabel3=请阅读以下许可协议。继续安装前，你必须接受本协议的条款。
LicenseAccepted=我接受协议(&A)
LicenseNotAccepted=我不接受协议(&D)

; ── 选择安装位置页 ──────────────────────────────────────────────────────────
WizardSelectDir=选择安装位置
SelectDirDesc=将 [name] 安装到哪里？
SelectDirLabel3=安装程序将把 [name] 安装到以下文件夹。
SelectDirBrowseLabel=要继续，请单击“下一步”。如果想换一个文件夹，请单击“浏览”。
DiskSpaceGBLabel=至少需要 [gb] GB 可用磁盘空间。
DiskSpaceMBLabel=至少需要 [mb] MB 可用磁盘空间。
CannotInstallToNetworkDrive=不能安装到网络驱动器。
CannotInstallToUNCPath=不能安装到 UNC 路径。
InvalidPath=请输入带盘符的完整路径，例如：%n%nC:\APP%n%n或者 UNC 路径：%n%n\\server\share
InvalidDrive=所选驱动器或 UNC 共享不存在，或无法访问。请另选一个。
DiskSpaceWarningTitle=磁盘空间不足
DiskSpaceWarning=安装至少需要 %1 KB 可用空间，但所选驱动器只有 %2 KB 可用。%n%n仍要继续吗？
DirNameTooLong=文件夹名称或路径过长。
InvalidDirName=文件夹名称无效。
DirExistsTitle=文件夹已存在
DirExists=文件夹：%n%n%1%n%n已经存在。仍要安装到该文件夹吗？
DirDoesntExistTitle=文件夹不存在
DirDoesntExist=文件夹：%n%n%1%n%n不存在。要创建该文件夹吗？

; ── 选择附加任务页 ──────────────────────────────────────────────────────────
WizardSelectTasks=选择附加任务
SelectTasksDesc=需要执行哪些附加任务？
SelectTasksLabel2=选择你希望在安装 [name] 时执行的附加任务，然后单击“下一步”。

; ── 准备安装页 ──────────────────────────────────────────────────────────────
WizardReady=准备安装
ReadyLabel1=安装程序已准备好，即将在你的电脑上安装 [name]。
ReadyLabel2a=单击“安装”开始安装；如果想检查或修改设置，请单击“上一步”。
ReadyLabel2b=单击“安装”开始安装。
ReadyMemoDir=安装位置：
ReadyMemoGroup=开始菜单文件夹：
ReadyMemoTasks=附加任务：
ReadyMemoUserInfo=用户信息：
ReadyMemoComponents=已选组件：
ReadyMemoType=安装类型：

; ── 安装前检查页（有文件被占用时出现）──────────────────────────────────────
WizardPreparing=准备安装
PreparingDesc=安装程序正在准备安装 [name]。
PreviousInstallNotCompleted=上一个程序的安装或卸载尚未完成。需要重启电脑才能完成，重启后再运行安装程序。
CannotContinue=安装无法继续，请单击“取消”退出。
ApplicationsFound=以下程序正在使用安装程序需要更新的文件，建议允许安装程序自动关闭它们。
ApplicationsFound2=以下程序正在使用安装程序需要更新的文件，建议允许安装程序自动关闭它们。安装完成后，安装程序会尝试重新启动这些程序。
CloseApplications=自动关闭这些程序(&A)
DontCloseApplications=不关闭这些程序(&D)
ErrorCloseApplications=安装程序无法自动关闭全部程序。建议你先手动关闭占用这些文件的程序再继续。
PrepareToInstallNeedsRestart=安装程序必须重启电脑。重启后再运行安装程序完成安装。%n%n要现在重启吗？

; ── 安装进度页 ──────────────────────────────────────────────────────────────
WizardInstalling=正在安装
InstallingLabel=正在安装 DraftFlow CAD，请稍候...
StatusClosingApplications=正在关闭应用程序...
StatusCreateDirs=正在创建文件夹...
StatusExtractFiles=正在解压文件...
StatusCreateIcons=正在创建快捷方式...
StatusCreateIniEntries=正在写入 INI 项...
StatusCreateRegistryEntries=正在写入注册表...
StatusRegisterFiles=正在注册文件...
StatusSavingUninstall=正在保存卸载信息...
StatusRunProgram=正在完成安装...
StatusRestartingApplications=正在重新启动应用程序...
StatusRollback=正在回滚更改...

; ── 完成页 ──────────────────────────────────────────────────────────────────
FinishedHeadingLabel=DraftFlow CAD 已就绪
FinishedLabel=准备改变你的绘图体验。
ClickFinish=单击“完成”退出安装程序。
FinishedRestartLabel=要完成 [name] 的安装，安装程序必须重启电脑。要现在重启吗？
FinishedRestartMessage=要完成 [name] 的安装，安装程序必须重启电脑。%n%n要现在重启吗？
YesRadio=是，现在重启电脑(&Y)
NoRadio=否，稍后我自己重启电脑(&N)
RunEntryExec=运行 %1
ShowReadmeCheck=是，我想查看 README 文件

; ── 卸载器（与安装向导共用 [Messages] 表，不覆盖就回退英文）──────────────────
ConfirmUninstall=确定要完全删除 %1 及它的所有组件吗？
UninstalledAll=%1 已成功从你的电脑中移除。
UninstalledMost=%1 已卸载完成。%n%n部分文件未能删除，你可以稍后手动删除。
UninstalledAndNeedsRestart=要完成 %1 的卸载，必须重启电脑。%n%n要现在重启吗？
UninstallStatusLabel=正在从你的电脑中移除 %1，请稍候...
UninstallDataCorrupted=“%1” 文件已损坏，无法读取卸载数据。

; ============================================================================
[CustomMessages]
DfAdditionalIcons=附加图标：
DfDesktopIcon=创建桌面快捷方式(&D)
DfLaunch=启动 {#MyAppName}

; ============================================================================
[Tasks]
; 文件关联是可选项：勾选才写注册表；不勾则系统里不留任何 .dwg/.dxf 关联
Name: "fileassoc"; Description: "将 .dwg 和 .dxf 关联到 DraftFlow CAD"; Flags: checkedonce
; checkedonce：首次安装默认勾选；用户取消过，升级时不再强行加回来
Name: "desktopicon"; Description: "{cm:DfDesktopIcon}"; GroupDescription: "{cm:DfAdditionalIcons}"; Flags: checkedonce

; ============================================================================
[Files]
; 主程序
Source: "{#MyAppExe}"; DestDir: "{app}"; DestName: "{#MyAppExeName}"; Flags: ignoreversion restartreplace
; 图纸类型专属图标：安装名固定为 dwg.ico / dxf.ico
Source: "{#DwgIcon}"; DestDir: "{app}"; DestName: "dwg.ico"; Flags: ignoreversion
Source: "{#DxfIcon}"; DestDir: "{app}"; DestName: "dxf.ico"; Flags: ignoreversion

; ============================================================================
[Icons]
; 开始菜单（全用户 → 公共开始菜单；单用户 → 当前用户开始菜单）
Name: "{group}\{#MyShortcutName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; Comment: "2D/3D CAD for DWG and DXF"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
; 桌面快捷方式（受上面 Tasks 控制）
Name: "{autodesktop}\{#MyShortcutName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; Tasks: desktopicon

; ============================================================================
;  HKA = 全用户安装写 HKLM，单用户安装写 HKCU，一套写法覆盖两种权限。
;  值名 “DraftFlow CAD” 必须与 src/io/file_association.rs 的
;  APP_REGISTRY_NAME 完全一致，否则应用内“设为默认”找不到自己。
;  ★ 所有文件关联项都带 Tasks: fileassoc —— 用户在任务页取消勾选时，
;    安装包不会写入任何 .dwg/.dxf 关联，只装程序本身。
[Registry]
; ── DWG ProgID（本安装包独占，卸载整键删除）────────────────────────────────
Root: HKA; Subkey: "Software\Classes\DraftFlowCAD.DWG"; ValueType: string; ValueName: ""; ValueData: "DWG Drawing"; Tasks: fileassoc; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\DraftFlowCAD.DWG\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\dwg.ico,0"; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\DraftFlowCAD.DWG\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: fileassoc

; ── DXF ProgID ──────────────────────────────────────────────────────────────
Root: HKA; Subkey: "Software\Classes\DraftFlowCAD.DXF"; ValueType: string; ValueName: ""; ValueData: "DXF Drawing"; Tasks: fileassoc; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\DraftFlowCAD.DXF\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\dxf.ico,0"; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\DraftFlowCAD.DXF\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: fileassoc

; ── 扩展名映射 ──────────────────────────────────────────────────────────────
; 默认值故意不加 uninsdeletevalue：卸载时由 [Code] 校验“仍指向本应用”才删，
; 避免用户改用别的 CAD 后，卸载把我们自己的关联以外的东西清掉。
Root: HKA; Subkey: "Software\Classes\.dwg"; ValueType: string; ValueName: ""; ValueData: "DraftFlowCAD.DWG"; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\.dxf"; ValueType: string; ValueName: ""; ValueData: "DraftFlowCAD.DXF"; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\.dwg\OpenWithProgids"; ValueType: string; ValueName: "DraftFlowCAD.DWG"; ValueData: ""; Tasks: fileassoc; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.dxf\OpenWithProgids"; ValueType: string; ValueName: "DraftFlowCAD.DXF"; ValueData: ""; Tasks: fileassoc; Flags: uninsdeletevalue

; ── “打开方式”里的应用条目 ─────────────────────────────────────────────────
Root: HKA; Subkey: "Software\Classes\Applications\DraftFlowCAD.exe"; ValueType: string; ValueName: "FriendlyAppName"; ValueData: "{#MyAppName}"; Tasks: fileassoc; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\Applications\DraftFlowCAD.exe\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\Applications\DraftFlowCAD.exe\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\Applications\DraftFlowCAD.exe\SupportedTypes"; ValueType: string; ValueName: ".dwg"; ValueData: ""; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\Applications\DraftFlowCAD.exe\SupportedTypes"; ValueType: string; ValueName: ".dxf"; ValueData: ""; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\Applications\DraftFlowCAD.exe\SupportedTypes"; ValueType: string; ValueName: ".bak"; ValueData: ""; Tasks: fileassoc

; ── 默认程序能力（“设置 → 应用 → 默认应用”里显示）──────────────────────────
Root: HKA; Subkey: "Software\DraftFlow CAD\Capabilities"; ValueType: string; ValueName: "ApplicationName"; ValueData: "{#MyAppName}"; Tasks: fileassoc; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\DraftFlow CAD\Capabilities"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "2D/3D CAD application for DWG and DXF drawings."; Tasks: fileassoc
Root: HKA; Subkey: "Software\DraftFlow CAD\Capabilities\FileAssociations"; ValueType: string; ValueName: ".dwg"; ValueData: "DraftFlowCAD.DWG"; Tasks: fileassoc
Root: HKA; Subkey: "Software\DraftFlow CAD\Capabilities\FileAssociations"; ValueType: string; ValueName: ".dxf"; ValueData: "DraftFlowCAD.DXF"; Tasks: fileassoc
Root: HKA; Subkey: "Software\RegisteredApplications"; ValueType: string; ValueName: "DraftFlow CAD"; ValueData: "Software\DraftFlow CAD\Capabilities"; Tasks: fileassoc; Flags: uninsdeletevalue

; ── App Paths（与文件关联无关，始终写入；Win+R 输入 DraftFlowCAD 可启动）──
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\App Paths\DraftFlowCAD.exe"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName}"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\App Paths\DraftFlowCAD.exe"; ValueType: string; ValueName: "Path"; ValueData: "{app}"

; ============================================================================
[Run]
; 完成页默认勾选“启动 DraftFlow CAD”；静默安装不启动
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:DfLaunch}"; Flags: nowait postinstall skipifsilent

; ============================================================================
[UninstallDelete]
; 可再生的图标缓存（便携/兜底路径才会产生，删掉无损失）
Type: filesandordirs; Name: "{localappdata}\DraftFlow CAD\file-icons"
; 目录空了就移除（不递归删用户可能放进去的文件）
Type: dirifempty; Name: "{app}"

; ============================================================================
[Code]
{ ============================================================================
  外观：全部交给 [Setup] 的原生深色样式（WizardStyle=modern dark）。
  本文件不含任何自绘上色代码 —— 自定义样式激活时 Pascal 设置的颜色
  会被样式覆盖，写了只会互相打架。
  ============================================================================ }
{ ============================================================================
  以下为安装/卸载逻辑（与外观无关）
  ============================================================================ }

{ 读取“配置目录曾被移动到哪里”：HKCU\Software\DraftFlowCAD 下 config_dir_<hash> }
function GetCustomConfigDir: String;
var
  Names: TArrayOfString;
  I: Integer;
  Value: String;
begin
  Result := '';
  if RegGetValueNames(HKCU, 'Software\DraftFlowCAD', Names) then
    for I := 0 to GetArrayLength(Names) - 1 do
      if Pos('config_dir_', Names[I]) = 1 then
        if RegQueryStringValue(HKCU, 'Software\DraftFlowCAD', Names[I], Value) then
        begin
          Result := Value;
          Exit;
        end;
end;

{ 只在扩展名默认值仍指向本应用时删除，绝不误删其他 CAD 的设置 }
procedure RemoveExtensionDefault(const Root: Integer; const Ext, ProgId: String);
var
  Current: String;
begin
  if RegQueryStringValue(Root, 'Software\Classes\' + Ext, '', Current) then
    if CompareText(Current, ProgId) = 0 then
      RegDeleteValue(Root, 'Software\Classes\' + Ext, '');
end;

{ 清理应用运行时自己写入的 HKCU 项（便携版 / 首次运行 / 应用内 FILEASSOC）。
  注意：只清理执行卸载的账户；其他用户的 HKCU 需各自清理，避免枚举 HKEY_USERS 的越权风险。
  本安装包不包含缩略图插件，因此不注册、也不清理 CLSID / ShellEx 相关项。 }
procedure CleanUserAssociations;
begin
  RegDeleteKeyIncludingSubkeys(HKCU, 'Software\Classes\Applications\DraftFlowCAD.exe');
  RegDeleteKeyIncludingSubkeys(HKCU, 'Software\Classes\DraftFlowCAD.DWG');
  RegDeleteKeyIncludingSubkeys(HKCU, 'Software\Classes\DraftFlowCAD.DXF');
  RegDeleteKeyIncludingSubkeys(HKCU, 'Software\Classes\DraftFlowCAD.BAK');
  RegDeleteKeyIncludingSubkeys(HKCU, 'Software\DraftFlow CAD');
  RegDeleteValue(HKCU, 'Software\RegisteredApplications', 'DraftFlow CAD');

  RegDeleteValue(HKCU, 'Software\Classes\.dwg\OpenWithProgids', 'DraftFlowCAD.DWG');
  RegDeleteValue(HKCU, 'Software\Classes\.dxf\OpenWithProgids', 'DraftFlowCAD.DXF');
  RegDeleteValue(HKCU, 'Software\Classes\.bak\OpenWithProgids', 'DraftFlowCAD.BAK');

  RemoveExtensionDefault(HKCU, '.dwg', 'DraftFlowCAD.DWG');
  RemoveExtensionDefault(HKCU, '.dxf', 'DraftFlowCAD.DXF');
  RemoveExtensionDefault(HKLM, '.dwg', 'DraftFlowCAD.DWG');
  RemoveExtensionDefault(HKLM, '.dxf', 'DraftFlowCAD.DXF');
end;

{ 安装前只提示一种冲突：之前用 MSI 装过同一款产品。
  Inno 自己的覆盖升级由 AppId 处理，不弹窗打扰。 }
function InitializeSetup: Boolean;
var
  Names: TArrayOfString;
  I: Integer;
  Key, DisplayName: String;
  IsMsi: DWORD;
begin
  Result := True;
  if RegGetSubkeyNames(HKLM, 'Software\Microsoft\Windows\CurrentVersion\Uninstall', Names) then
    for I := 0 to GetArrayLength(Names) - 1 do
    begin
      Key := 'Software\Microsoft\Windows\CurrentVersion\Uninstall\' + Names[I];
      if RegQueryStringValue(HKLM, Key, 'DisplayName', DisplayName) then
        if CompareText(DisplayName, 'DraftFlow CAD') = 0 then
          if RegQueryDWordValue(HKLM, Key, 'WindowsInstaller', IsMsi) and (IsMsi = 1) then
          begin
            if MsgBox('检测到之前用 MSI 安装的 DraftFlow CAD。' + #13#10 +
                      '继续安装会写入新的程序目录，可能留下两份程序。' + #13#10 +
                      '建议先在“设置 → 应用”里卸载旧版。是否仍要继续？',
                      mbConfirmation, MB_YESNO) = IDNO then
              Result := False;
            Exit;
          end;
    end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  DefaultDir, CustomDir: String;
begin
  { 注册表与文件关联：无论是否保留配置都必须清理 }
  if CurUninstallStep = usUninstall then
    CleanUserAssociations;

  { 卸载完成前询问是否保留用户数据 }
  if CurUninstallStep = usPostUninstall then
  begin
    { 静默卸载（/VERYSILENT）不弹窗打扰，按“保留”处理，绝不擅自删数据 }
    if UninstallSilent then
      Exit;
    DefaultDir := ExpandConstant('{userappdata}\DraftFlowCAD');
    { 先读出自定义目录，删除位置记录后就查不到了 }
    CustomDir := GetCustomConfigDir;

    if MsgBox('是否保留 DraftFlow CAD 的配置和设置？' + #13#10 + #13#10 +
              '是：保留（推荐）。只卸载程序、快捷方式和文件关联。' + #13#10 +
              '否：删除默认配置目录 ' + DefaultDir + ' 和位置记录。' + #13#10 + #13#10 +
              '注意：你自定义到其他盘位的字体、打印样式、自动保存目录不会被删除。',
              mbConfirmation, MB_YESNO) = IDNO then
    begin
      DelTree(DefaultDir, True, True, True);
      RegDeleteKeyIncludingSubkeys(HKCU, 'Software\DraftFlowCAD');
      if (CustomDir <> '') and (CompareText(CustomDir, DefaultDir) <> 0) then
        MsgBox('你的配置目录曾在：' + #13#10 + CustomDir + #13#10 + #13#10 +
               '该目录不在默认位置，卸载程序不会删除它，请按需手动清理。',
               mbInformation, MB_OK);
    end;
  end;
end;
