# Q-Dir 的 Rust 重写报告

> 工程目录：`D:\L-dir\qdir-rs`（edition 2024，cargo 1.95.0 / rustc 1.95.0）
> 原程序：`D:\Q-Dir_Portable_x64\Q-Dir\Q-Dir_x64.exe`，PE32+ x86_64，sha256 `321d3288cddf37373fb2519711f8ab569cd5a2f9bf9813142de9a05da509a40c`，4034 个函数、1945 个字符串、57 个 RT_DIALOG、10 个 RT_MENU。
> 本报告全部结论来自本会话内实际执行的命令与对源码的逐行阅读；凡未验证的，一律写入「未覆盖」。

---

## 0. 一句话结论

**地基与全部业务子系统（core / config / panes / filters / services / shell，约 1.39 万行 Rust）已写完，`cargo check --lib` / `--bin qdir-rs` / `--tests`、`cargo test`（224 lib + 6 config 集成 + 5 core 集成，全绿，0 warning）、`cargo build --release`（产出 15.1 MB 的 `target/release/qdir-rs.exe`）全部通过；但图形界面层 `src/ui/*` 从未创建，`src/main.rs` 仍是一个只显示状态摘要的 eframe 占位应用，四窗格、菜单、标签页、拖放接收、托盘、设置界面在二进制里均不存在，shell/panes/services/filters 的公开 API 在 `src/` 内零生产调用者——因此当前能编译能运行，但还远远不是一个"能用的 Q-Dir"。**

---

## 1. 构建与运行

以下命令全部在本会话于 `D:\L-dir\qdir-rs` 下实际执行：

| 命令 | 结果 |
|------|------|
| `cargo check --lib` | `Finished dev profile`，0 error 0 warning |
| `cargo check --bin qdir-rs` | `Finished dev profile`，0 error 0 warning |
| `cargo check --tests` | `Finished dev profile`，0 error 0 warning |
| `cargo test --lib` | `224 passed; 0 failed; 0 ignored` |
| `cargo test`（全套） | 224 + 0 + 6 + 5 通过，1 个 doc-test `ignored`，0 failed |
| `cargo build --release` | `Finished release profile [optimized]`，3m46s，0 warning，产出 `target/release/qdir-rs.exe`（15 129 600 字节） |

测试分布（`cargo test --lib` 按模块计数）：shell 66、panes 63、services 33、filters 28、core 21、config 13，合计 224。

运行：

```bat
cd D:\L-dir\qdir-rs
cargo run --release
```

运行时会读工作目录下的 `Q-Dir.ini` 与 `start.qdr`（`src/main.rs:63-70`），缺失则各自取默认值；真实错误才 `eprintln` 后 `exit(1)`。当前窗口是一个 800×600（取自证据 `WinRC=0;0;800;600`，`src/main.rs:15`）的 egui 窗口，显示「地基就绪」与会话摘要（`src/main.rs:36-53`）。

依赖（`Cargo.toml:6-14`）：eframe/egui 0.37、windows 0.62（13 个特性）、csv、image 0.25、time 0.3、fluent-langneg 0.14、zip 9。其中 **fluent-langneg 在 `src/` 内零使用**（grep 确认），Win32_System_Registry 特性同样零使用。

---

## 2. 架构与模块划分

依赖方向 D1：`core` 最底层，`config` 依赖 core，其余模块只依赖 core（契约规定），`lib.rs` 只做 `pub mod` 登记（`src/lib.rs:10-15`）。

| 模块 | 文件 | 行数 | 职责 | 对应原程序哪部分 |
|------|------|------|------|------------------|
| core | `src/core.rs` | 1721 | 全部领域类型：FileItem / PaneId / ViewMode / SortKey / TabState / PaneState / ColumnConfig / SplitPermille / SessionState / Config / RuleSet / ExportFormat / Rgb / QError，视图状态串 `ViewMode,%d;ImageSize,%d;FolderFlags,%d;GroupBy,%s;Group,%d;` 的生成与解析（`src/core.rs:896-935`） | 四窗格数据模型、视图状态串、Filter2 规则模型、.qdr 会话字段 |
| config | `src/config.rs` | 1588 | Q-Dir.ini 与 start.qdr（均 UTF-16LE）的解析/回写、IniDocument 保留未知键与顺序、`set_in_place`（E2 不新增键）、`ensure_modeled_keys` | 原版 ini/qdr 序列化行为、[Start]/[Favoriten]/[Q]/[Preview]/[Ordner]/[Filter2] 全部段 |
| filters | `src/filters/mod.rs` | 1389 | Filter2 着色求值（`resolve(item,row_index)→(fg,bg)`）、5 类名称过滤器（by_filter/by_dir_filter 双作用域、`;` 与 `,` 双分隔符）、`Column_OS_10.0_Ploder1..4` 十六进制 blob 解析 | 着色规则、通配符过滤、列配置持久化格式 |
| panes | `src/panes/mod.rs` `tree.rs` `preview.rs` | 1255 / 704 / 464 | PaneController（进入/后退/前进/上级、`select_last_if_navigate_up`、`refresh_keep_sel/scroll`、四种视图模式与排序、列宽、过滤串、标签页语义）、懒加载目录树（tree_mode/single_expand/auto_expand_from_list/auto_sel/indent_width 生效）、三类预览数据（图片 RGBA / 文本前 N 行 / 二进制前 256 字节） | 四窗格 Ploder1..4、目录树、预览面板 |
| services | `src/services/`（export 693 / favorites 596 / i18n 533 / mod 18） | 1840 | 五种导出（TXT/CSV/HTML/SpreadsheetML-XLS/Tabgetrennt、UTF-8 BOM、列可配、`{Q-DIR-FILE-PATH}` 模板变量）、Favoriten/Quick-Link 枚举增删改名分组（Fav_Filter 过滤）、约 120 条 de/en/zh 三语表（`src/services/i18n.rs:72-217`，131 条 Entry） | 菜单 216 的导出/收藏夹/窗口文案、RT_MENU 德语基线 |
| shell | `src/shell/`（mod 356 + 9 个文件 4073） | 4429 | Windows Shell 集成：目录枚举（FindFirstFileW / IShellFolder）、HICON→RGBA、SHFileOperationW 文件操作、IContextMenu 两步式右键菜单、拖入拖出（手写 COM vtable）、CF_HDROP 剪贴板、特殊文件夹与驱动器、Shell_NotifyIconW 托盘 | SHELL32/COMCTL32/ole32/KERNEL32/comdlg32/WINSPOOL/USER32/GDI32 导入表对应的能力 |

**未创建的模块**：`src/ui/`（mod/layout/filelist/dialogs/toolbar）、`src/app.rs`、`src/layout.rs`、`src/pane.rs`、`src/dirtree.rs`、`src/filelist.rs`、`src/rules.rs`、`src/favorites.rs`（模块化后改为 services/favorites.rs）、`src/export.rs`、`src/print.rs`、`src/i18n/*`——模块清单里登记的 UI 层一个都没有落盘。

---

## 3. 功能对照表

状态定义：**已实现**=代码存在且本会话验证过其逻辑路径；**部分实现**=有代码但语义/能力缺口（缺口逐条列出）；**未实现**=代码不存在。**「可运行」列指该能力在 `cargo run --release` 的产物里是否真的可达。**

| 证据功能 | 状态 | 代码位置 | 可运行 | 说明 |
|---|---|---|---|---|
| 四窗格 Ploder1..4 布局 | 未实现 | — | 否 | 数据模型齐（`core::PaneId::ALL`），无任何渲染/窗格容器代码；main.rs 是单 CentralPanel 占位 |
| start.qdr 分割条 SP0/1/2=5000/5003/5003、SPT=2000 | 已实现 | `src/core.rs` SplitPermille、`src/config.rs:1215-1227` | 只读 | 千分比换算往返；万分数→千分数有 3/10000 取整损失（已注释登记，config.rs:1210-1214） |
| Vorschau / VorschauPos=7500 预览开关与位置 | 部分实现 | `src/core.rs` SessionState、`src/config.rs:1229-1230` | 否 | 字段解析/回写齐；preview.rs 只产数据，面板显隐无渲染 |
| Focus=1 活动窗格 | 部分实现 | `src/core.rs` SessionState.focus、config.rs:1231 | 否 | 落盘/读回齐；无窗格切换交互 |
| colorthema=0 配色主题 | 未实现 | 仅字段 config.rs:1232 | 否 | 无主题求值/应用代码 |
| 会话 .qdr 读写（UTF-16LE，Folder1..4 含 IDLIST 与 ViewModeFlags/ViewMode/UNICODE 子字段） | 已实现 | `src/config.rs:1207-1264`、`QdrFolder` | 是（库侧） | 真实文件往返测试通过：`tests/config_tests.rs::qdr_byte_identical_roundtrip_on_real_file` 等 6 项 |
| tree_mode / Tree1..4 / Tree1..4split=2500 | 部分实现 | config.rs:1228/1237-1238；`src/panes/tree.rs` | 否 | 解析与树状态机齐（63 个测试），无树控件渲染 |
| Q-Dir.ini [Start] 段（QDir_Id/useColorStart/Als/designe_mode/showCmd/chk_ini/StartCrash/WinRC） | 已实现（读/写） | `src/core.rs:1183-1189`、config.rs:736-738 | 只读 | 全部建模；除 main.rs 读 WinRC/标题外无消费方（无设置界面） |
| [Favoriten] Ordner / [Q] Link / [Q-Dir] Lizenz | 已实现 | config.rs:581-582、752-753；`src/services/favorites.rs` | 否 | 目录枚举/增删改名/分组齐；Lizenz 未建模 |
| [Preview] Filter=*; / [Ordner Filter] | 部分实现 | `src/core.rs:1194`（同一 `preview_filter` 字段）、`src/filters/mod.rs` NameFilters.preview | 否 | core 未为 [Ordner] Filter 单独建模（已登记差异），两段同名键共用一字段 |
| [Filter2] 12 条着色规则（含 #Hidden/#DIR/#BG/#BG-A 特殊模式与通配符） | 已实现 | `src/core.rs` RuleSet、`src/filters/mod.rs` ColorResolver | 否 | 逐字节往返测试见 `tests/core_tests.rs::filter2_evidence_twelve_rules_parse_and_roundtrip`；仅 rule 4 回写把 `,` 统一为 `;`（语义等价，已登记） |
| 列配置 Column_OS_10.0_Ploder1..4（772 字节 blob） | 部分实现 | `src/filters/mod.rs:303-620` | 否 | 只解出 GUID 锚定的列条目表（4 条，宽 24/180/225/144，PropGroup→Ext、Keywords→Attr）；IconSize/FFlags/排序方向因 blob 内 1 字节错位与 size/vlen 矛盾无法取证，回落默认值（H2(a) 已登记） |
| 视图模式：大图标/小图标/列表/详情（菜单 216 德语原文） | 部分实现 | `src/core.rs:200-205` ViewMode；i18n.rs:116-119 | 否 | 枚举与状态串齐（63 个 panes 测试覆盖切换语义）；无菜单无渲染 |
| 视图状态串 `ViewMode,%d;ImageSize,%d;...` | 已实现 | `src/core.rs:896-905` | 是（库侧） | 生成与解析双向，测试断言 `ViewMode,3;ImageSize,32;FolderFlags,1075838981;GroupBy,Ext;Group,1;` |
| 全部配置键（tree_* 13 个、tab_* 8 个、copy_cut_paste_mode、move_file_confirmation、refresh_*、select_last_if_navigate_up、columns_plus_view、colorthema、langid、FontFaceName、LangName、show_filter、last_filter、by_dir_filter、by_filter、Markierungsfilter、Last-File-Filter、Fav_Filter） | 部分实现 | `src/core.rs:1160-1240` Config、`src/config.rs:705-720` | 否 | **键全部建模且能读能写**，但「在设置里可改」不成立：无设置 UI。Markierungsfilter 连求值器都没有（`src/filters/mod.rs:289-290` 注释称由 filelist 另行求值，而 filelist 不存在）；columns_plus_view / group_by 解析出来但零消费方，分组功能实际缺失 |
| 菜单 216：&Druck / &Seite Einrichten / Spalten &Anpassen / &Linien anzeigen / Liste &Exportieren / 列宽适应文本 | 未实现 | 仅 i18n 文案（i18n.rs:93-105、113-115） | 否 | 无菜单栏代码；eframe 窗口无 MenuBar。文案（德语逐字取自 RT_MENU）与实现脱节 |
| 导出 TXT/CSV/HTML/XLS/Tabgetrennt | 已实现 | `src/services/export.rs` | 否 | 五种格式 + UTF-8 BOM + 列可配 + `{Q-DIR-FILE-PATH}`（export.rs:470）；33 个 services 测试。**XLS 是最简 SpreadsheetML 2003 而非真实 BIFF**（原版 XLS 内部格式未取证，H2(c)） |
| &Favoriten / Zu Favoriten hinzufügen / &Verwalten / Im neuem Fenster mit Strg | 部分实现 | `src/services/favorites.rs:149-250` | 否 | 增删改名/分组/Fav_Filter 过滤齐；**Quick-Link 无法导航**：`favorites.rs:328-342` 只按扩展名分类为 `FavoriteKind::Lnk`，从不解析 .lnk 目标路径，shell 层也无 IShellLinkW |
| &Fenster → 层叠/并排/上下/排列图标 | 未实现 | 仅 i18n.rs:127-130 | 否 | 无任何实现代码 |
| 目录枚举（列文件） | 未实现（缺陷） | `src/shell/enumerate.rs:79-103` | 否 | **致命缺陷**：见 §5.1。`list_directory` / `list_directory_detailed` 对一切目录恒失败，四窗格列不出任何文件 |
| SHGetFileInfoW 系统图标 | 部分实现 | `src/shell/icons.rs` | 否 | `icon_image_for_path` 真机验证成功（16×16，本会话实测）；`image_for_path`/`image_for_index` 因 `load_system_icon` 恒失败而不可用（icons.rs:247-258），见 §5.2 |
| SHFileOperationW 复制/移动/删除/回收站 | 已实现 | `src/shell/fileops.rs:96-200` | 否 | 主路径 SHFileOperationW + FOF_ALLOWUNDO 回收站 + 系统进度框；兜底 CopyFileW/MoveFileW/DeleteFileW/RemoveDirectoryW（fileops.rs:389-411） |
| 打印（&Druck、&Seite Einrichten） | 未实现 | `src/shell/fileops.rs:267-269` | 否 | `print_file` 只是 `ShellExecuteW("print")`；无 PrintDlgW / OpenPrinterW / GetPrinterW 调用（grep 全仓零命中） |
| IContextMenu 右键菜单 | 部分实现 | `src/shell/contextmenu.rs` | 否 | 两步式 QueryContextMenu/InvokeCommand + track 齐；但 `install_menu_hook`（contextmenu.rs:445-450）只做一次无副作用 `cast::<IContextMenu2>()`，**模块文档（contextmenu.rs:15）声称的 `SetWindowsHookEx(WH_MSGFILTER)` 全文不存在**，WM_INITMENUPOPUP/MEASUREITEM/DRAWITEM 转发实际不可用 |
| 拖入拖出 | 部分实现 | `src/shell/dragdrop.rs` | 否 | DragAcceptFiles/DragQueryFileW/DragFinish + 手写 IDropSource vtable + ole32 原始 DoDragDrop 绑定（dragdrop.rs:280-390、428-560）；**未接入任何窗口过程**（无 WM_DROPFILES 处理者），`begin_drag` 除单测外零调用者 |
| 剪贴板 | 部分实现 | `src/shell/clipboard.rs` | 否 | CF_HDROP + Preferred DropEffect 读写齐（OleSetClipboard）；CFSTR_SHELLIDLIST（虚拟项）解析未实现，读取走 CF_HDROP |
| 特殊文件夹 / 驱动器枚举 | 已实现 | `src/shell/special.rs` | 否 | SHGetSpecialFolderPathW、桌面/文档/回收站/此电脑/库路径、驱动器枚举（special.rs:146） |
| Shell_NotifyIconW 托盘 | 部分实现 | `src/shell/tray.rs` | 否 | NIM_ADD/气球/版本/tip 截断齐（tray.rs:93-200）；`TrayIcon::new` 除单测外零调用者，无托盘消息路由，最小化到托盘/单击还原未接入 |
| 注册表扩展 `\shell\*Q-Dir` | 未实现 | — | 否 | Cargo.toml:13 声明 Win32_System_Registry，`src/` 内零处使用（grep 确认） |
| ImageList_* | 未实现 | — | 否 | 模块声明提及，icons.rs:243 注释说明「SHGetFileInfoW 路径已覆盖且更简单」，未使用 |
| comdlg32 ChooseColorW / GetOpenFileNameW | 未实现 | — | 否 | grep 全仓零命中 |
| WINSPOOL OpenPrinterW / GetPrinterW | 未实现 | — | 否 | grep 全仓零命中 |
| SHBrowseForFolderW | 未实现 | — | 否 | grep 全仓零命中 |
| 多语言（language 1031 资源） | 部分实现 | `src/services/i18n.rs` | 否 | 三套静态表（de/en/zh，131 条）；**无加载 language 1031 资源/语言 DLL 的机制**；fluent-langneg 依赖零使用。部分德语项资源里是数字占位符（10001、51005），用合理译名补齐并在表内标注来源类别 |
| 启动耗时 `Q-Dir v12 %0d msek` | 未实现 | — | 否 | 无计时/状态栏 |
| `Q-Dir_Tmp\`、`color.ini` | 未实现 | — | 否 | grep 零命中 |
| `{Q-DIR-FILE-PATH}` 模板变量 | 已实现 | `src/services/export.rs:470` | 是（库侧） | `expand_template` 有测试 |
| TrayNotifyWnd / UnhandledExceptionFilter | 未实现 | — | 否 | 无崩溃处理/托盘类名相关代码 |
| 设置保存（save_config/save_session） | 部分实现 | `src/config.rs:1282、1305` | 否 | 函数存在且真实文件往返测试通过（tests/config_tests.rs:64/86/119/177），**但全仓无生产调用者**（grep：仅 4 处测试调用），用户修改不会落盘 |
| 打开默认程序 / 属性对话框 | 已实现 | `src/shell/fileops.rs:210-264` | 否 | ShellExecuteW("open"/"properties")；同样无调用者 |

---

## 4. 与原程序的行为差异与刻意简化

**配置持久化**
1. `ensure_modeled_keys`（`src/config.rs:882-1015`）只补写「值 != Config::default()」的键。把某键改回默认值时，`set_in_place` 会保留原文，导致该修改静默丢失（例：langid 从 1033 改回 1031 不生效）——复核员指出的用户数据丢失风险，代码行为与该判断一致（config.rs:369-380、882-884）。
2. `apply_session_to_doc`（`src/config.rs:1237-1263`）对 TreeN/FolderN/ColsAndSortN 无条件用 `doc.set` 而非 `set_in_place`，与「不新增键」的声明不一致：缺这些键的 .qdr 会被改写结构；实测证据文件键齐全，故往返测试通过。
3. 万分数→千分数换算有 3/10000 取整损失（SP=5003 → 500），已在注释中登记为量纲固有上限。

**列配置 blob**
4. 772 字节封装有 1 字节错位（key 落在记录起点+9）且 size 与 vlen 互相矛盾，IconSize/FFlags 单个字段无法推出 value 偏移——按契约降级为「只解析 GUID 锚定、无歧义的列条目表」，其余回落默认值（`src/filters/mod.rs:485-496`）。
5. 列标识是「GUID 族 → core 列名」的粗映射（只见到 PropGroup→Ext、Keywords→Attr 两个族），不是真实列名；原版向 Windows Shell 询问显示名，重写版无此依赖。
6. `ColInfo` 之后、条目表之前有 26 字节容器自有头部（含 u32=120、0xFDFDFDFD、16、4）含义未取证；实现用「从 value 末尾向后扫描到第一个可识别 GUID」绕过，没写死偏移。
7. 排序方向未解出（Sort 记录的 value 是 `prop4294967295`，只编码了默认排序列），按 `SortDir::Asc` 回落。

**着色与过滤**
8. rule 4 的 `,` 分隔符回写时统一为 `;`（语义等价，测试已锁定差异）。
9. [Ordner] Filter 与 [Preview] Filter 在 core 里是同一个 `preview_filter` 字段（ini 两个段同名键），未单独建模。
10. Markierungsfilter 无求值器——依赖不存在的 filelist。

**预览**
11. MAX_IMAGE_DIM=512、MAX_TEXT_BYTES=1MiB、DEFAULT_MAX_TEXT_LINES=200、BINARY_PREVIEW_BYTES=256、MAX_PREVIEW_FILE_BYTES=64MiB 全部是约定值，未从二进制取证（`src/panes/preview.rs:26-45`）。
12. 图片/文本扩展名表是约定值。
13. 目录树只维护「以当前目录为根」的一棵树；原版可从桌面展开到盘符。深层刷新是浅层替换（懒加载语义，测试已锁定）。
14. 选择集/滚动位置/焦点行进不了 `core::PaneState`（契约形状固定且 .qdr 无对应键），放在 `panes::PaneViewState`，不参与序列化与 PartialEq 往返。

**导出与收藏夹**
15. XLS 是最简 SpreadsheetML 2003（XML 表格），非真实 BIFF；Excel/WPS/LibreOffice 可打开。
16. 不创建真正的 Windows .lnk（需 IShellLinkW COM）；Quick-Link 因此不可导航。
17. 收藏夹分组约定为「Favoriten 下一级子目录即分组、菜单展平显示组内项」，未逐字节取证。
18. FILETIME 按 UTC 输出，原版显示本地时间。
19. i18n 部分德语项资源里是数字占位符，用合理译名补齐并标注来源类别；中英文为人工翻译。

**Shell**
20. 按系统图标索引（iIcon）取 HICON 的可靠公开 API 未找到（需系统图像列表句柄），`load_system_icon` 恒返回 Err 交由上层兜底；日常路径 `icon_image_for_path` 直接拿 HICON。
21. IDropSource/IDropTarget/IDataObject::SetData 三处无法用 windows crate 安全封装（`#[implement]` 宏生成 `::windows_core::` 绝对路径而 windows-core 非直接依赖；SetData gate 在未启用的 Win32_System_Com_StructuredStorage 后），改为手写 COM vtable + ole32 原始 `link!` 绑定（dragdrop.rs:280-390、428-560）。
22. 剪贴板虚拟项（CFSTR_SHELLIDLIST）解析未实现。
23. 注册表扩展 `\shell\*Q-Dir` 属安装期行为，未实现。

**UI 层**
24. 完全没有 UI：无四窗格、无分割条拖动、无菜单、无标签页条、无拖放接收、无托盘、无设置界面、无打印/页面设置对话框。`src/main.rs` 是「最小 eframe 应用」占位（main.rs:20-55）。
25. 窗口排列（层叠/并排/上下/排列图标）、Spalten Anpassen、Linien anzeigen、列宽适应文本等菜单项只有文案。

---

## 5. 已知问题与后续建议

### 5.1 目录枚举 100% 失效（致命，已本会话实测复现）

`src/shell/enumerate.rs:79-89` 与 `92-103`：`dir_with_backslash()`（`src/shell/mod.rs:189-196`）返回的是**已带 NUL 结尾**的宽串（`to_wide` 在 `mod.rs:150-154` push 了终结 0），随后代码又 `push(b'*' as u16); push(0);`——`'*'` 被追加到 NUL 之后，作为 C 字符串传给 `FindFirstFileW` 时只剩目录名本身。

我做了两级验证。第一级，独立复现该内存布局（`rustc` 直调 FindFirstFileW，程序在 `%TEMP%\fftcheck\main.rs`）：

```
实际传入的 C 字符串 = "C:\Windows\"
A 代码实际产物: 失败 GetLastError=2
B 双星号: 成功 first="" attrs=0x00000010
C 正确单星号: 成功 first="" attrs=0x00000010
```

（注：复核员描述的 `C:\Users\**` 双星号与该代码的实际产物不同——`'*'` 落在 NUL 之后而非之前。但**结论一致且更糟**：实际传入的是无通配符的纯目录名，恒返回 ERROR_FILE_NOT_FOUND。本报告只采信实测。）

第二级，直接调用 qdir-rs 的公开 API（外部 crate 经 `shellprobe` 引用 `D:/L-dir/qdir-rs`）：

```
list_directory(C:\) => Err: Shell 错误：FindFirstFileW：HRESULT 0x80070002 系统找不到指定的文件。
list_directory(D:\) => Err: ... 0x80070002 ...
list_directory(C:\Windows) => Err: ... 0x80070002 ...
list_directory(C:\Users\liuxinlong\AppData\Local\Temp\) => Err: ... 0x80070002 ...
list_directory_detailed(C:/Windows) => Err: ... 0x80070002 ...
```

四个目录全部失败，文件枚举这一唯一入口 100% 失效。**修法**：把 `'*'` 写到 NUL 之前（替换最后一个码元），或让 `dir_with_backslash` 返回不带终结 0 的 Vec 再由调用方补全。这条不修，任何 UI 接入后都列不出文件。

### 5.2 GDI 对象泄漏（已本会话实测确认）

`src/shell/icons.rs:457-464`：`mem_dc_for()` 用 `CreateCompatibleDC` 创建的**源** DC，在 `icons.rs:402` 被直接传进 `BitBlt` 后丢弃，从未 `DeleteDC`（目标 mem_dc 在 432 行有释放，源 DC 没有）。

实测（1000 次 `icon_image_for_path`，前后 `GetGuiResources(-1, GR_GDIOBJECTS)`）：

```
GDI 对象: 前=42 后=2042 增量=2000
每调用泄漏对象数 ≈ 2.00
```

每调用泄漏 2 个 GDI 对象（复核员说 2.04，实测算术平均 2.00）。列表每渲染一次都会触发，长时间运行将耗尽 GDI。

### 5.3 `image_for_path` / `image_for_index` 恒失败（已本会话实测确认）

`src/shell/icons.rs:247-258`：`load_system_icon` 无条件返回 Err，导致文档宣称的主路径 `IconCache::image_for_index` 永远失败。实测：

```
image_for_path(C:\Windows\notepad.exe) => Err: Shell 错误：不支持按系统索引 353 直接取图（Small），请改用 image_for_path
image_for_path(C:\Windows) => Err: Shell 错误：不支持按系统索引 0 直接取图（Small），请改用 image_for_path
image_for_index(3) => Err: Shell 错误：不支持按系统索引 3 直接取图（Small），请改用 image_for_path
icon_image_for_path => Ok 16x16
```

即 `FileItem.icon_index` 无法转成纹理，只有 `icon_image_for_path` 真能出图。**建议**：用 `SHGetFileInfoW(SHGFI_PIDL | SHGFI_SYSICONINDEX)` 拿 PIDL 后走 `IImageList`（需系统图像列表句柄，可从 `SHGetImageList` 或 `FileIconInit` 后取的 shell 图像列表获得），或在 UI 层统一改走 `icon_image_for_path` 并按扩展名缓存（`IconCache::image_for_ext` 已是这个形状）。

### 5.4 右键菜单 hook 不存在

`src/shell/contextmenu.rs:15` 的模块文档声称「用 `SetWindowsHookEx(WH_MSGFILTER)` 装一个线程内 hook」，但全文（grep 确认）无 `SetWindowsHookEx`；`install_menu_hook`（contextmenu.rs:445-450）只做了一次无副作用的 `cast::<IContextMenu2>()`。WM_INITMENUPOPUP/MEASUREITEM/DRAWITEM/MENUCHAR 转发依赖不存在的窗口过程。**建议**：要么实现 hook（安装/卸载生命周期与菜单存活期绑定），要么删掉文档里的承诺；在 egui 自绘菜单的架构下，更现实的是对 owner-draw 扩展项降级为「仅注入普通项」。

### 5.5 UI 层整体缺失

四窗格、分割条拖动、四种视图渲染、菜单（含 RT_MENU 216 全部德语项）、标签页条、拖放接收、托盘、设置界面、打印/页面设置、窗口排列均不存在。shell/panes/services/filters 的公开 API 在 `src/` 内零生产调用者（grep 确认）。这是「能否算重写完成」的决定性缺口。

### 5.6 保存不落盘

`save_config`/`save_session`（config.rs:1282、1305）全仓无生产调用者（仅 4 处测试调用）。即便 UI 接入，用户修改也不会写回 Q-Dir.ini / start.qdr。

### 5.7 其它（复核员指出、本会话已核对代码但未实机运行）

- 设置界面不存在，因此「全部配置键在设置里可改」不成立；Markierungsfilter 连求值器都没有。
- 打印能力未实现：`print_file` 只是 `ShellExecuteW("print")`，无 PrintDlgW/OpenPrinterW/GetPrinterW。
- `\shell\*Q-Dir` 注册表扩展未实现（Registry 特性声明了但零使用）。
- 窗口排列未实现（只有 i18n 四条文案）。
- 多语言只有 De/En/ZhCn 三套静态表，无 language 1031 资源/语言 DLL 加载机制；fluent-langneg 零使用。
- Quick-Link 不可导航（.lnk 只按扩展名分类，从不解析目标）。
- 拖放与托盘未接入窗口过程：`accept_drop_files`/`begin_drag`/`TrayIcon::new` 除单测外无调用者。
- `ensure_modeled_keys` 的「改回默认值即丢失」问题（见 §4.1）。
- `apply_session_to_doc` 对 TreeN/FolderN/ColsAndSortN 用 `doc.set` 破坏「不新增键」（见 §4.2）。
- `IconSize` 只支持 16/32/48 三档，无法承载 `ImageSize,%d` 的任意值。
- `ColumnConfig::group_by` / `columns_plus_view` 解析出来但无消费方，分组功能实际缺失。

### 5.8 建议的优先级顺序

1. **修 enumerate 的 NUL 拼接**（5.1）——不修则 UI 接入后完全不可用，一行改动。
2. **修 mem_dc_for 泄漏**（5.2）——在 `hicon_to_rgba` 的清理块里补 `DeleteDC` 源 DC。
3. **建 `src/ui/` + `src/app.rs` 并接入 shell/panes/services/filters**（5.5）——当前最大的功能缺口；同时接上保存（5.6）、拖放、托盘、右键菜单。
4. **补 load_system_icon 或统一改走 icon_image_for_path**（5.3）。
5. 补打印对话框（PrintDlg + WINSPOOL）、窗口排列、注册表扩展、语言 DLL 加载。
6. 修配置回写语义（5.7 最后两条）。

---

## 6. 未覆盖 / 未验证清单

- **未运行 GUI**：本会话只跑了 `cargo build --release` 产出二进制，没有启动 `qdir-rs.exe` 验证窗口外观或交互（报告结论基于 main.rs 源码与 eframe 0.37 API 签名）。
- **单测不触达真实 Windows API**：224 个 lib 测试除少量失败即跳过的探测外，全部是纯逻辑；单测全绿不代表 Windows 行为正确——5.1/5.2/5.3 三个缺陷恰恰是单测覆盖不到的部分。
- **列 blob 未逐字节取证**：IconSize/FFlags/排序方向/26 字节容器头部的含义未解出（H2(a)）。
- **原版 XLS 内部格式未确认**（H2(c)）： SpreadsheetML 与原版是否行为一致未比对。
- **托盘点击还原、拖放落点、菜单弹出、四窗格渲染、分割条拖动**：全部未实测（无实现）。
- **多语言资源 1031 加载机制**：未实现也未验证。
- **Q-Dir v12 msek 启动耗时、Q-Dir_Tmp\、color.ini、TrayNotifyWnd、UnhandledExceptionFilter、SHBrowseForFolderW、ChooseColorW、GetOpenFileNameW、PrintDlgW、OpenPrinterW、GetPrinterW**：grep 确认全仓零命中，属未实现而非未验证。
