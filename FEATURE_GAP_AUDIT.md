# L-Dir 功能完备性证据审计（FEATURE_GAP_AUDIT）

> 审计日期：2026-10-23。只读审计，未修改任何源码。
> 原版参照：`D:\Q-Dir_Portable_x64\Q-Dir\Q-Dir_x64.exe`（12.71，sha256 321d3288…a40c）。
> 证据源：① `assets/menu_texts_zh.txt`（运行时菜单文案实测）② `NATIVE_SPEC.md` 附录 A
> （Ghidra 确证行为表）③ `D:\qdir_reverse\decompiled\all_functions.c`（行号下文记作 `AF:行`）
> ④ `assets/extracted/4/*` 原版菜单模板 ⑤ 原版目录样本。
> L-Dir 现状证据：`src/` 下 `文件:行号`。标 **\*待复核\*** = 推断而非字节确证。

## 0. 方法与勘验记录

- 原版枚举以「菜单文案 + 模板 id + 附录 A 行为表」交叉为准；本次补充对 AF 的
  API 串扫（结果：AF 含 SHFileOperationW×5、RegisterDragDrop×6、DoDragDrop×2、
  SHDoDragDrop×3、IDropTarget 族、SHChangeNotifyRegister×3（AF:19522/97253/180492，
  消息 0x400/0x10f0）、SHAutoComplete×3（AF:72/51599/199020）、CreateMutexW×1
  （AF:168214，名含 "Q-DIR-GUID"）、SetClipboardData/OleSetClipboard×8、
  LVM_GETEDITCONTROL(0x101b→本表记 0x1032? 否——`0x1032`=LVM_GETSELECTEDCOUNT) 等。
  **未见** 字符串 Everything / MultiName / SHEmptyRecycleBin / IContextMenu（原版
  的 shell 菜单是 WTL 经 SHGetPathFromIDList+IContextMenu COM 调用，伪 C 以
  vtable 偏移出现，grep 串名不可见——相关判定一律 \*待复核\*）。
- 勘验修正一处：原版权威菜单快照第 79 行「撤消 删除(U)\tCtrl+Z」——撤销对象是
  **删除**（与文件操作对话框语义一致），不是通用撤销栈；AF 未见 undo 串互证，\*待复核\*。
- L-Dir 行号均为本次 grep 实测。

---

## 表 1：功能总清单（原版证据 → L-Dir 状态 → 缺口 → 严重度）

### 1.1 导航与窗格

| # | 功能 | 原版证据 | L-Dir 现状 | 缺口 | 级别 |
|---|---|---|---|---|---|
| 1 | 四窗格列目录 | 窗格结构（REFACTOR/UIA 实测） | `mainframe.rs:1358 fill_pane`→`list_directory_detailed`（`enumerate.rs`，NUL 拼接缺陷已修） | 无 | ✅ |
| 2 | 双击进入/Backspace 上跳/地址栏回车 | 原版惯例 | `subclass.rs:138-149`（Enter/Back/addr 回车）；`mainframe.rs:1756/1789/1819` | 无 | ✅ |
| 3 | 历史导航（后退/前进，窗格内历史） | 原版 ViewMenu+history | `mainframe.rs:583-585,620-622 tab.history`（历史链已记录） | 后退/前进**无 UI 触发**（按钮/鼠标中键/Alt+←未接）\*待复核\* | P1 |
| 4 | 导航历史下拉（ComboBox/go 按钮 0xd20） | 附录 A.4 `go.png(0xB379)` | 窗格有 ComboBox（`mainframe.rs:2159-2163`）仅驱动器列表；**CBN_SELCHANGE 未处理**（全文件只 CB_ADDSTRING/CB_SETCURSEL，`mainframe.rs:2412/2418`） | 下拉选盘不导航；无历史下拉 | P1 |
| 5 | 每窗格独立视图模式 | 附录 A.5（0x7029–0x702C→活动窗格） | `set_view_mode_all`（`mainframe.rs:1085`）——**全窗格一刀切** | 原版也是全窗格?（A.5 称"活动窗格"）按活动窗格逐格化未做 | P2 |

### 1.2 标签页

| # | 功能 | 原版证据 | L-Dir 现状 | 缺口 | 级别 |
|---|---|---|---|---|---|
| 6 | 标签切换 | 原版 | `TCN_SELCHANGE→tab_select`（`mainframe.rs:1637-1644`） | 无 | ✅ |
| 7 | 中键/双击关标签（开关决策） | 配置键 close_by_mbt_clk 等 | `subclass.rs:79-126`+`core::tab_click_action`（含拒绝关最后一个） | 无 | ✅ |
| 8 | 新建标签 Ctrl+T | menu_texts:41「新建标签页 Ctrl+T」 | **无任何 dispatch/加速键**（app.rs:106 加速键表仅 F3）；`PaneState::add_tab` 现成（`mainframe.rs:2291` 测试钩子在用） | 命令+加速键缺失 | **P0** |
| 9 | 在新标签页打开（选中/双击目标） | menu_texts:39,42 | 缺失（同上没有 add_tab 接线） | 命令缺失 | P1 |
| 10 | 重复标签页 Ctrl+K | menu_texts:40 | 缺失 | 命令缺失 | P2 |
| 11 | 标签空白双击新建 | 配置键 `tab_new_tab_l_dblclk/clk`（`config.rs:90-91`，仅建模） | subclass 双击只处理"在标签上"，空白区新建未接 | 键模了行为没接 | P2 |
| 12 | 标签右键菜单 | 配置键 `tab_use_context_menu`（`config.rs:97`） | 未接 | 缺失 | P2 |
| 13 | show_tabs 开关 | 附录 A.8 0x8558/0x8559 | config 有、UI 无消费 | 设置未生效 | P3 |

### 1.3 剪贴板与文件操作

| # | 功能 | 原版证据 | L-Dir 现状 | 缺口 | 级别 |
|---|---|---|---|---|---|
| 14 | 复制/剪切/粘贴（操作本体） | AF SetClipboardData×8、CF_HDROP 语义\*待复核\* | `shell/clipboard.rs` 全套 + 窗格按钮 bi=5/6/7（`commands.rs:448-470`） | 操作本体可用（按钮路径） | ✅ |
| 15 | **键盘 Ctrl+C/X/V** | menu_texts:43（选择性粘贴）+惯例 | **未接**：subclass.rs 无 WM_KEYDOWN(C)处理；原生 ListView 默认 WM_COPY 只复制文本标签非 CF_HDROP\*待复核\* | 键盘剪贴板全套缺失 | **P0** |
| 16 | Ctrl+A 全选 / Ctrl+Shift+A 反选 | menu_texts:37-38 | **未接**（LVM_SETITEMSTATE 只在焦点移动/规则框使用，`mainframe.rs:1177/2928`） | 缺失 | **P0** |
| 17 | Del 删除 | menu_texts 无独立项；惯例+视图菜单「粘贴/撤消」 | **未接键盘**；删除本体=按钮 bi=4（`commands.rs:438-446`） | Del 键缺失 | **P0** |
| 18 | 删除进回收站 | 视图菜单「撤消 删除 Ctrl+Z」（menu_texts:79）→原版删除默认 FOF_ALLOWUNDO\*待复核\*（AF 的 5 处 SHFileOperationW 只见 fFlags=0x40/0，未见显式 0x40=ALLOWUNDO 判定，AF:65207 段 0x40 亦可能是 FOF_WANTNUIDEFALOG——语义二选一，**待复核**） | `delete_files` 支持 allow_undo（`fileops.rs:149,409`）但调用侧 `FileOpOptions::default()`→**allow_undo=false=永久删除**（`fileops.rs:49-58`） | 删除不落回收站、不可撤销 | **P0** |
| 19 | 撤消删除 Ctrl+Z | menu_texts:79 | 未接（无命令、无加速键；回收站撤销依赖 #18 改 ALLOWUNDO 后由系统兜底即可） | 缺失 | P1 |
| 20 | F2 重命名 | menu_texts:68「重命名(M)」；AF PostMessage(0x100,0x71)（AF:63506/106679=向窗格发 WM_KEYDOWN/F2 转发，重命名入口确证\*机制待复核\*） | **完全未接**：无 LVS_EDITLABELS（`mainframe.rs:36 LVS_REPORT_SHOWSEL` 不含）、无 LVM_EDITLABEL、无 begin/edit 通知；`fileops.rs:161 rename_path` 本体在但仅 egui 版调用（`ui/app.rs:844`） | 原生 UI 无重命名 | **P0** |
| 21 | 新建文件夹 | Q-Dir 常规入口（工具栏/右键；附录 A.1 已证文件菜单无此项——走 shell 背景右键菜单） | 原生 UI **无入口**（`new_folder` 仅 egui `ui/app.rs:825`）；shell 背景右键菜单可代理系统"新建文件夹"（`pane_context_menu` 空白区，`mainframe.rs:407-424`）——但**非窗格内即时新建** | 原生菜单/快捷键无新建文件夹 | **P0**（*若确认背景右键够用可降 P1*） |
| 22 | 选择性粘贴 Ctrl+Shift+V | menu_texts:43 | `paste_into` 只有普通粘贴（`commands.rs:464`） | 冲突处理对话框缺失 | P2 |
| 23 | 创建快捷方式 | menu_texts:67 | shell 右键菜单代理可用；无自有命令 | 低 | P3 |
| 24 | 复制路径 | menu_texts:54 | shell 右键菜单代理可用（Win11 自带）；无自有命令 | 低 | P3 |
| 25 | 释放磁盘空间 | menu_texts:36 | 未接（`fileops.rs` 无 clean-manager 调用；原版疑起 diskmgmt/cleanmgr\*待复核\*） | 缺失 | P3 |
| 26 | 清空回收站 | AF 未见 SHEmptyRecycleBin 串\*待复核\*；`fileops.rs:375-391` 封装了 SHQueryRecycleBin/SHEmptyRecycleBin | **未接 UI** | 封装闲置 | P3 |

### 1.4 拖放（标志性交互）

| # | 功能 | 原版证据 | L-Dir 现状 | 缺口 | 级别 |
|---|---|---|---|---|---|
| 27 | 拖出到 Explorer/外部 | AF:26887 DoDragDrop、:107517 SHDoDragDrop×3、:180483 RegisterDragDrop | `dragdrop.rs:170 begin_drag`（CFSDROPFILELIST+PreferredDropEffect 完备）——**ui_native 零调用**（grep `begin_drag` 仅命中本模块与测试） | 拖出完全未接 | **P0** |
| 28 | 窗格间拖放（A→B 复制/移动） | 同上+IDropTarget 族（AF:26380/32055/113619/114630/150733 多处注册） | **无 IDropTarget 实现**；`register_drop_target`/`accept_drop_files`（`dragdrop.rs:113/395`）闲置 | 未接 | **P0** |
| 29 | 从 Explorer 拖入窗格 | 同上 | 未接（`DragAcceptFiles` 无人调用；mainframe 无 WM_DROPFILES 处理） | 未接 | **P0** |
| 30 | 窗格按钮「移动到下一窗格」 | 原版按钮 bi=0/1 语义（附录留档） | `commands.rs:423-424 move_selection_to_next` | 有替代通道（非拖拽） | ✅ |

### 1.5 Shell 上下文菜单

| # | 功能 | 原版证据 | L-Dir 现状 | 缺口 | 级别 |
|---|---|---|---|---|---|
| 31 | 列表右键=Shell 菜单 | menu_texts:47-69 动态项 | `NM_RCLICK→pane_context_menu→ContextMenu::query_and_run`（`mainframe.rs:1610-1613,407`；`contextmenu.rs:336`）；IContextMenu2/3 菜单消息 hook 已有（`contextmenu.rs:439-446`） | 无（打开方式/属性/7-Zip 等自动继承） | ✅ |
| 32 | 树/标签/托盘右键 shell 菜单 | 原版树右键 | 仅列表接了；树/标签未接 | 部分 | P2 |

### 1.6 视图

| # | 功能 | 原版证据 | L-Dir 现状 | 缺口 | 级别 |
|---|---|---|---|---|---|
| 33 | 四种视图模式切换 | 附录 A.5 | `commands.rs:334-341` LVM_SETVIEW 0/2/3/1 | 消息已发但见 #34 | ✅(有条件) |
| 34 | **大图标视图出图** | 原版 IconSize/image_size 键（`core.rs:903`） | 列表只挂 **LVSIL_SMALL**（`mainframe.rs:2269-2279`），**无 LVSIL_NORMAL 大图标图像列表**（全 src 无 `LVSIL_NORMAL`/`SHGFI_LARGEICON`→列表调用；`icons.rs:56` 封装闲置） | 切到大图标≈无图\*待复核（需实测）\* | **P0**（用户清单点名） |
| 35 | 列排序（点列头） | 原版 | `subclass.rs:57-73` HDN_ITEMCLICK→apply_column_sort | 无 | ✅ |
| 36 | 列显隐（0x80C0–0x80CB→`Als` 位） | 附录 A.8 `FUN_0045b114` | 未接（config 解析 ViewMode 串含 FFlags 但无列开关 UI/行为） | 缺失 | P1 |
| 37 | 排序方式/分组依据子菜单 | menu_texts:75-76；原版经 `IContextMenu`/shell 组视图\*待复核\*（AF 未见 LVM_INSERTGROUP 串——组视图由 LVM_SETGROUPINFO 系 COM，伪 C 串不可见） | `core.rs:895 group_by` 仅 .qdr 串编解码；无排序/分组菜单 | 缺失 | P1 |
| 38 | 网格线/列宽自适应 | 附录（32864/33938-39） | `commands.rs` VIEW_GRIDLINES/FIT_COL\*（menu.rs:32-34） | 无 | ✅ |
| 39 | 预览面板 | 原版 Vorschau | PRODUCT_SPEC §3 P2 ✅（三路刷新） | 无 | ✅ |
| 40 | 快速过滤 + F3 | menu_texts:24「查找 F3」 | `commands.rs:208-212` F3 聚焦过滤框+EN_CHANGE 筛名 | 无 | ✅ |
| 41 | 隐藏/系统文件显示开关 | 原版选项-文件夹段 0x8019–0x802B（附录 A.8 folders 族） | `fill_pane` **硬编码 include_hidden:true**（`mainframe.rs:1369-1371`），无开关 | 缺失（含"显示扩展名"未见） | P1 |
| 42 | 全屏 F11 | menu_texts:73 | 未接 | 缺失 | P2 |
| 43 | 缩放 60–200% | 原版 | ZOOM_BASE 11 档（menu.rs:47,65） | 无 | ✅ |
| 44 | 配色 5 档 | 原版 | THEME_BASE+apply_theme_palette | 无 | ✅ |
| 45 | 数据大小信息 F9 | menu_texts:102 | 未接（状态栏有容量段 `mainframe.rs:930` 但无 F9 对话框） | 缺失 | P2 |

### 1.7 树

| # | 功能 | 原版证据 | L-Dir 现状 | 缺口 | 级别 |
|---|---|---|---|---|---|
| 46 | tree_mode 三态+懒加载+reveal | 附录 A.3 | PRODUCT_SPEC P1.3 ✅（端到端实测在案） | 无 | ✅ |
| 47 | 树着色三档 useTreeColor（0x413-415） | 附录 A.3 | 未接（src 无 tree_color） | 缺失 | P2 |
| 48 | 树跟随选择/整行选择等开关 | 附录 A.8 tree_auto_sel/tree_fullrowselect | 设置对话框已含并即时生效（PRODUCT_SPEC） | 无 | ✅ |

### 1.8 布局 / 状态 / 槽

| # | 功能 | 原版证据 | L-Dir 现状 | 缺口 | 级别 |
|---|---|---|---|---|---|
| 49 | 12 档布局切换 | 附录 A.4 | `splitter::Layout::ALL`+LAYOUT_BASE 弹层+z6/v6 循环（PRODUCT_SPEC P1.2 ✅） | 无 | ✅ |
| 50 | 状态 .qdr 打开/保存/存桌面/重复窗口 | 附录 A.1 | `commands.rs:95-137`（FILE_OPEN/SAVE/SAVE_DESKTOP/DUP_WINDOW=存临时 qdr+新实例） | 无 | ✅ |
| 51 | **桌面布局槽 0xB798–0xBB80**（单击载入/Ctrl 复制路径/Shift 另存/Del 删除） | 附录 A.7 `FUN_00474a34` | **未接**（src 无对应 B798 段命令、无槽表） | 整套缺失 | P1（Q-Dir 招牌，但属进阶用户） |
| 52 | 重启/管理员重启/关于 | 附录 A.1/A.6 | `commands.rs:222-228`、`restart_self`(:672)、ABOUT | 无 | ✅ |
| 53 | 单实例（互斥体） | AF:168214 CreateMutexW("Q-DIR-GUID"+GUID)，命中已存在时的转发逻辑\*待复核\* | **未接**（src 无 Mutex/FindWindow 单实例逻辑；`main.rs` 直接起窗） | 二次启动开两窗，且文件关联/拖入转发无落点 | P1 |
| 54 | 命令行参数 | 原版接 .qdr/路径/多实例旗标（`-help` 串见 AF:168211） | `main.rs:34-44` **只认第一参且必须 .qdr 结尾**（大小写敏感、目录路径不接） | 缺口：目录作参、`/s /m` 类旗标 | P1 |
| 55 | 关机对话框 | 附录 A.2（IShellDispatch 槽 0xA0） | FILE_RUN 已接槽 0x80（`commands.rs:152-158`），**0xA0 关机分支未接**（无 FILE_SHUTDOWN 命令 id） | 缺失 | P3 |
| 56 | 窗口置顶 | 原版选项\*待复核（AF 未见 TOPMOST 串） | 未接 | 缺失 | P2 |
| 57 | 语言切换（13 组 locale 表） | 附录 A.6 0xE147 | 未接（自有 i18n 表 `services/i18n.rs` 已有词条） | 缺失 | P3 |

### 1.9 收藏夹

| # | 功能 | 原版证据 | L-Dir 现状 | 缺口 | 级别 |
|---|---|---|---|---|---|
| 58 | 添加到收藏夹（.lnk） | menu_texts:85；Favoriten 目录 | `commands.rs:239-253` FAV_ADD（create_shortcut） | 无 | ✅ |
| 59 | 管理收藏夹 | menu_texts:86 | FAV_MANAGE=用系统默认打开收藏夹目录（`commands.rs:255-259`）——**非原版自有管理对话框** | 简化实现 | P2 |
| 60 | Quick-Link 弹层 | 原版按钮 bi=9 | `commands.rs:472`→`show_quick_link_menu`（`mainframe.rs:990-1054`） | 无 | ✅ |
| 61 | Ctrl 新 Q-Dir 打开收藏夹项 / Shift 覆盖 / Del 删除 | menu_texts:90-91 | 未接（弹层仅单击打开） | 修饰键族缺失 | P2 |

### 1.10 系统集成

| # | 功能 | 原版证据 | L-Dir 现状 | 缺口 | 级别 |
|---|---|---|---|---|---|
| 62 | **文件夹监视自动刷新** | AF:19522/180492 `SHChangeNotifyRegister(...,0x201ff,0x400,...)`、:97253 `(...,0x6201f,0x10f0,...)`（双注册：树 0x400/列表 0x10f0） | **未接**（src 无 SHChangeNotifyRegister；VIEW_REFRESH 手动刷新已有 `commands.rs:229-232`） | 目录变化不自动刷新 | P1 |
| 63 | 地址栏自动补全 | AF SHAutoComplete×3（延迟绑定 shell32） | 未接（`autocomplete` 配置键附录 A.8 有、src 无） | 缺失 | P1 |
| 64 | 托盘最小化/还原/右键 | 原版\*行为未反编译确证\* | `mainframe.rs:380-390` SC_MINIMIZE 拦截+托盘（PRODUCT_SPEC ✅ 端到端在案） | 无 | ✅ |
| 65 | 开机恢复 | 原版\*待复核\* | `shell/startup` Run 键+设置复选 ✅ | 无 | ✅ |
| 66 | 查找（文件搜索对话框） | menu_texts:24 F3+选项-查找 | L-Dir 的 F3=聚焦过滤框（简化）；原版 F3 疑为独立搜索对话框\*待复核\* | 口径差异 | P2 |
| 67 | Everything 集成 | menu_texts:46 | 未接（src 无；AF 亦无 Everything 串——原版为外部进程调用，机制\*待复核\*） | 缺失 | P3 |
| 68 | 导出 TXT/CSV/HTML/XLS | 附录 A.2 0x8030 | `commands.rs:160-196` ✅ | 无 | ✅ |
| 69 | 打印 | menu_texts:28 | `commands.rs:197-207` print 动词 | 无 | ✅ |
| 70 | 添加网络位置 / 网络驱动器 / 默认浏览器 / 浏览 / 重新定位窗口 | menu_texts:80,110,112,26,29 | 未接 | 缺失 | P3 |
| 71 | 帮助菜单族（在线帮助/FAQ/捐赠/mailto） | 附录 A.6 | ABOUT 已接；其余 URL 族未接 | 缺失 | P3 |
| 72 | 多名字视图 MultiNameView | 任务书提及，**AF 未见串** | 原版是否存在待证 | **\*待复核\***（建议查 IMG 语言表键） | — |

### 1.11 设置持久化

| # | 功能 | 原版证据 | L-Dir 现状 | 缺口 | 级别 |
|---|---|---|---|---|---|
| 73 | Q-Dir.ini 全键读写往返 | REFACTOR/NATIVE_SPEC §3"已 100% 实现" | `config.rs`（1663 行，键族/ensure_modeled_keys/往返单测） | 个别键已建模未消费（#13,#41,#63） | ✅ |
| 74 | 设置对话框 | 原版 57 个 DLGTEMPLATE | 自建 `settings_dialog.rs`（15 项三组即时生效）——覆盖面小于原版页签群（界面/布局/列表/树/标题栏/地址栏/颜色/状态栏/标签/文件操作/打印/预览/字体/查找/查看设置/系统/网络…共 21 子页，menu_texts:94-114） | 多数原版设置页无对应 | P2 |
| 75 | 高亮/着色规则对话框 | 原版规则族（Filter2/#Hidden 等） | `rules_dialog.rs` ✅ | 无 | ✅ |

---

## 表 2：P0 不可丢核心功能逐项判定

（用户给定的日常必用清单；「判定」= 可用性而非代码存在性）

| 核心功能 | 判定 | 说明与证据 |
|---|---|---|
| 列目录 | ✅ 可用 | #1 |
| 导航（进入/上跳/地址栏） | ✅ 可用 | #2；历史后退缺 UI 触发（P1，不判丢） |
| 标签 | ⚠️ **半残** | 切换/关闭可用（#6,#7）；**新建/在标签打开完全无入口**（#8,**P0**）——用户无法新增标签，只减不增 |
| 剪贴板全套 | ❌ **键盘路径全缺** | 仅窗格按钮可 C/X/V（#14）；**Ctrl+C/X/V/A/Del 均未接**（#15,#16,#17,**P0**） |
| 拖放 | ❌ **全缺** | 拖出/窗格间/外部拖入三条路径均未接 UI（#27-29,**P0**）；封装代码闲置 |
| 右键菜单 | ✅ 可用 | #31（列表项+空白背景两态） |
| 重命名 | ❌ **缺** | F2/菜单/内联编辑全无（#20,**P0**）；rename_path 本体在、无调用方 |
| 删除+回收站 | ⚠️ **可删不可撤** | 删除经按钮可用但**永久删除**（allow_undo=false 默认，#17/#18,**P0**）；Del 键缺；Ctrl+Z 缺（#19,P1） |
| 新建文件夹 | ❌ **缺原生入口** | 仅背景 shell 右键代理可达（#21,**P0**，验收需实测背景"新建"在窗格路径下确实生效——若生效可降 P1，标\*待复核\*） |
| 地址栏 | ✅ 可用 | 回车导航 #2；补全 P1（#63）、下拉选盘未接 P1（#4） |
| 过滤 | ✅ 可用 | #40 |
| 视图模式含大图标 | ⚠️ **大图标疑无图** | LVM_SETVIEW 已发但无 LVSIL_NORMAL（#33/#34,**P0**，需运行时实测坐实） |
| 设置持久化 | ✅ 可用 | #73 |

**P0 缺口汇总（6 项）**：①键盘剪贴板全套（Ctrl+C/X/V/A/Del）②重命名（F2+内联编辑）
③拖放三路径 ④新建文件夹入口 ⑤删除默认回收站+Ctrl+Z ⑥标签新建/大图标图像列表（渲染坐实后定级）。

---

## 总体判断

- 按表 1 的 75 项计：✅ 已实现 ≈ **37 项**（49%），部分实现 ≈ 9 项，缺失/未接 ≈ 29 项。
- 但 ✅ 项集中在"外壳骨架"（窗格/布局/树/持久化/对话框），**缺失项集中在"文件系统操作主干"**
  （键盘操作、拖放、重命名、新建、回收站）——按日常使用频率加权，核心工作流完成度约 **60%**，
  总功能面加权约 **52–58%**。
- 好消息：所有缺失的 P0 项在 `shell/` 层**均有已测封装**（clipboard/fileops/dragdrop/contextmenu），
  缺的是 ui_native 接线（subclass.rs 键盘分发、dragdrop 注册、菜单命令与加速键），
  属"临门一脚"型工作量；真正需要新写的只有 IDropTarget COM 对象、LVS_EDITLABELS 内联编辑链、
  大图标图像列表与监视回调消息泵。
- 最大风险：`begin_drag` 的 DoDragDrop 是**阻塞式**且允许效能为单值（cut?copy:move 二选一），
  接窗格间拖放需扩为 copy|move 双许可 + 按键效果解析，接线时须复核（dragdrop.rs:250-252）。
