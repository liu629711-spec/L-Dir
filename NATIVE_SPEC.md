# Q-Dir 逆向规格——**参考档案**（历史证据汇总）

> ⚠️ **状态变更（2026-10-21 战略转向）**：项目已放弃 1:1 复刻目标，转向
> 做 Q-Dir 同类产品（布局体验/交互对标、界面外观自主）。本文档不再是
> 验收规格来源，而是**逆向证据存档 + 行为需求来源**：
> - 产品总纲、规则、功能清单、开发计划见 **PRODUCT_SPEC.md**。
> - 本文「附录 A：命令行为总表」是功能清单的证据来源，仍在使用。
> - 「资源字节复用 / 语言表解码 / 像素/UIA diff」等 1:1 目标**已废止**
>   （原版资产不得进产品二进制，见 PRODUCT_SPEC 规则 R1）。
> - 历史证据、*待复核* 声明原样保留，供追溯。

目标：`D:\Q-Dir_Portable_x64\Q-Dir\Q-Dir_x64.exe`（sha256 321d3288…a40c，Delphi VCL，4034 函数）。
本文档原为原生 UI 层（windows-rs）实现的唯一规格来源。核心逻辑层（core/config/filters/services）已实现并测试通过，**保持不变**。

## 1. 已提取的原始资源（assets/extracted/，277 个文件）

| 目录 | 内容 | 1:1 用法 |
|---|---|---|
| `5/` | 57 个 DLGTEMPLATE(EX) 对话框模板（原始二进制） | 用 `CreateDialogIndirectParamW`/`DialogBoxIndirectParamW` **原样创建**——字体、坐标、控件 id 与原版像素级一致 |
| `4/` | 10 个 MENUEX/MENU 模板（128=菜单栏, 216/217/303/305/307/319/358/430/4161） | `LoadMenuIndirectW` **原样加载**——菜单结构/快捷键/帮助 ID 与原版一致（文本占位符运行时按语言表替换） |
| `14/` | 30 个图标组（.ico 原始数据） | 作为应用/托盘/窗口图标直接注册 |
| `IMG/` | 40 个语言资源（Delphi 自定义压缩，头 `FEFF 1698`） | 压缩格式未破解；1:1 替代方案见 §5 |
| `2/ 3/ 240/ 241/` | 光标/位图/加速键/版本资源 | 加速键表用 `LoadAccelerators` 等价数据结构重建 |
| `PNG/ WAVE/ ZIP/` | 工具栏图/提示音/内嵌包 | PNG 作工具栏图像源，WAVE 事件音，ZIP 内嵌帮助包 |

## 2. REA 定锚（行为规格的可继续深挖锚点）

- **WinMain = `0x4e5f20`**：`CoInitialize` → `OleInitialize` → `InitCommonControlsEx(dwICC=0x6FF)`（列表/树/页签/热键/Animate/Rebar 类）→ VCL Application 初始化 → 主窗体运行。复刻时 dwICC 必须一致。
- **分割条核心 = `0x42a3bc`**（已反编译）：
  - 布局结构体字段：`+0x24` 当前分割像素位置、`+0x3c` 分割条宽度、`+0x1c/+0x14` 或 `+0x18/+0x2c` 两侧窗格边界；
  - **比例存储 = `MulDiv(pos, 10000, clientWidth - splitterWidth)` —— 万分比**，与 .qdr 的 SP0/SP1/SP2/SPT 完全对应；
  - 拖动钳制逻辑、`flags bit0(bit 0x1)=比例模式 / bit2=反向模式` 已恢复；`0x42a228` 是重绘触发。
- **timer 窗口类 = `0x4b7d84`**：类名 `Test_Top_Win32________timer_class%d`，WndProc=`0x4b6bdc`，用于隐藏消息窗口。
- VCL 主窗口类名是运行时构造（`TApplication` 派生），非静态字符串——复刻时用同类名规则 `TApplication.<标题>`。
- **Q-Dir Colors 背景刷 = `0x4bc484`**（已反编译）：`CreateBitmap(16,16,1,32, 像素表) → CreatePatternBrush → DeleteObject(bmp)`；像素表按参数色逐 4 字节填充 256 项 + 8×8 的 0xFFFFFFFF 交错区——这就是列表背景纹理的精确生成算法（含 color.ini 主题色）。
- **资源→GDI+ 位图加载器 = `0x4bc530`**（已反编译）：`FindResourceW(module,name,type)` → `LoadResource/LockResource` → `GlobalAlloc(GMEM_MOVEABLE)` 拷贝 → `CreateStreamOnHGlobal` → `GdipCreateBitmapFromStreamICM`。语言表 IMG 与 PNG 工具栏图都走这条路——**IMG 资源在运行时先经 `FUN_0050dc70`(memcpy) 拷贝后进 GDI+，说明 IMG 内容本身是完整图像/数据流；压缩解压发生在更上游（写入资源时的构建期压缩），运行时若 GDI+ 解析失败才有自定义解压回退**。复刻策略：IMG 资源字节原样嵌入，按此调用链加载；若 GDI+ 失败则回落自定义语言表。


## 3. 会话/配置格式（已 100% 实现，不再重复）

Q-Dir.ini / start.qdr 的 UTF-16LE 读写、12 条 Filter2 规则、SP 万分比、Focus、VorschauPos——均在 `src/config.rs` + `src/core.rs`，235 个测试通过。

## 4. 语言表（IMG 资源）结论

40 个 IMG 资源 = 40 种语言（ARA/CHS/…），**自定义压缩流**（头 `FE FF 16 98`，熵 11.9 bit/word，非 zlib/gzip/bz2/lzma）。

**运行时加载链（已反编译）**：`FUN_0047c790(lang_out, name)`
1. `FindResourceW(hInstance, name, "IMG")` → `LoadResource` → `LockResource` → `SizeofResource`；
2. 拷入栈缓冲（上限 `0x6B800` 字节）；
3. **`FUN_004793f4(out_stream, in_stream, DAT_00546e70, 0)` = 解压器入口**（这就是需要逆向的压缩算法宿主）；
4. 解压产物是宽字符串（Delphi WideString，头部 -8 偏移存长度）——即语言 ini 文本。

**复刻策略（二选一，按工作量）**：
- A（推荐）：逆向 `FUN_004793f4` 的解压算法（LZ 类，待做），在 Rust 端复刻，然后 IMG 资源字节原样嵌入 `.res`，运行时 `FindResourceW("IMG")` + 自研解压 → 与原版 100% 同源；
- B（过渡）：构建期用原版程序批量解出 40 份语言 ini（让原版 Q-Dir 导出），转为明文资源嵌入——文本 1:1，字节不同源。

## 5. 原生 UI 层架构（windows-rs）

```
src/ui-native/
  app.rs        消息循环 + WinMain 等价物（ICC=0x6FF、CoInit/OleInit 顺序一致）
  mainframe.rs  主窗口：TApplication 类名规则、菜单 128/216 加载、WM_COMMAND → 命令分发
  splitter.rs   按 0x42a3bc 规格：万分比存储、拖动钳制、双缓冲重绘
  panes.rs      四窗格容器（原生 ListView LVS_REPORT/LIST/ICON/SMALLICON + TreeView）
  tabs.rs       标签页（CCM tab control + tab_* 配置语义）
  dialogs.rs    57 个模板经 CreateDialogIndirectParamW 加载（资源内嵌 .res）
  shellui.rs    IContextMenu/拖放/托盘/打印对话框接线
```

关键原则：**资源即界面**——对话框/菜单/图标从提取的二进制原样重建为 `.res` 链接进 exe，不重新手绘；行为差异只在"消息处理代码"这一层，由 §2 的锚点逐个反编译补齐。

## 6. 与 egui 版的关系

egui 版（src/ui/）保留为对照参考，不删除；bin 入口切换到 ui-native。

## 验证工具链（已建成，可重复）

- `examples/control_tree.rs <PID> [out.json]` — UI Automation 控件树 dump（字段固定 → 字节稳定可 diff）
- `examples/enum_children.rs <PID>` — Win32 EnumChildWindows 原始视角（rect/可见性/类名）
- `examples/menu_probe.rs` — 菜单模板直接加载探针（独立进程验证字节合法性）
- `examples/menu_probe2.rs` — 完整初始化顺序（Co/Ole/ICC）下的菜单加载探针

对照方法：原版与重写各自 dump 控件树 → 按层级 diff → 逐项补齐。

## 已知偏差（原版 vs 重写，2026-10-20 v7 dump）

| 项 | 原版 | 重写 | 说明 |
|---|---|---|---|
| ListItem | 53 | 552 | 原版只暴露**可见行**；重写灌了全量行。语义等价，虚拟模式后自然对齐 |
| Text (no class) | 172 | 2220 | 同上：原版每行单元格按需暴露可见区 |
| SplitButton | 22 | 4 | 原版列头每窗格 4 个 SplitButton + 2 全局；重写 4 窗格各 1（对象计数），列头是 HeaderItem |
| Edit (no class) | 53 | 0 | 原版行内名称单元格 Edit（可见行）；重写用 ListItem 文本 |
| ToolBar | 14 | 12 | 缺全局 2 个挂接 UIA（Win32 已建，被 Type50037 占位吞）；窗格 12 个已齐 |
| ReBarWindow32 | d1 | Win32 有、UIA 树无 | 同上，UIA 占位问题 |
| WTL_XSplitterWindow | 8 | 0 | 原版分割条是专用窗口类；重写用自绘 SplitterGroup（行为等价，待补专用类名） |
| TabItem | 5 | 4 | 原版某窗格有 2 页签；重写每窗格 1 页签（会话数据如此） |
| MenuItem | 7/10 | — | 原版 6 弹出菜单可见（模板后 3 个运行时被删）；重写 9 个全挂。待按运行时删除逻辑对齐 |

## 工程化决定（已固化）

1. **菜单/对话框模板：原字节原样**交给 LoadMenuIndirectW / CreateDialogIndirectParamW（重建版本曾导致访问违例）。
2. **LoadMenuIndirectW 偶发 NULL**（GetLastError==0，连续启停进程时复现）：5 次重试 + 50ms 间隔绕过。
3. **窗格容器类名** `ATL:00000000004189CC` 在宿主进程内注册等价类（DefWindowProc 包装）。
4. **主窗口类名** `ATL:0000000000546C90` 与原版 1:1。
5. 分割条退化保护：SP 序列非严格递增时回退等分布局。

## 菜单命令分发（2026-10-20 完成首轮）

`src/ui_native/commands.rs`：WM_COMMAND 数值 id → shell 层操作。已接并端到端验证：

| id | 语言键 | 命令 | 验证 |
|---|---|---|---|
| 57601 | 10002 | 复制 | ✓ COPY 后外部读回剪贴板 `C:\Users\liuxinlong\.agents` |
| 57603 | 10003 | 剪切 | 同 COPY 链路（cut 标志不同） |
| 32820 | 10004 | 粘贴 | shell::paste_into + 重灌窗格 |
| 45912 | 205 | 删除 | shell::delete_files + 重灌窗格（未实测，避免删用户数据） |
| 45877 | 204 | 打开 | ShellExecuteW open |
| 45913 | 206 | 属性 | ShellExecuteW properties |
| 57600 | 10001 | 新建文件夹 | ✓ `新建文件夹 2` 于 15:34:12 落盘，避开已有重名 |
| 34037 | 53011 | 刷新 | fill_pane 重枚举 |
| 57664 | 60001 | 关于 | 原版 DLG 100 原字节弹框 |

未识别 id 留痕打印（`未接命令 id=...`），按原版行为逐个补齐。

验证工具链更新：
- `examples/verify_commands.rs <PID> <cmdId>`——外部只发数值 WM_COMMAND（跨进程安全）。
- 重写版内部钩子 `QDIR_TEST_SELECT_ROW=<行号>`：填充时自动选中该行（同进程 SendMessageW 传指针安全）。
  生产环境不设该变量即无影响。
- **教训**：跨进程 `SendMessageW` 传本地结构指针会崩目标进程（LVM_SETITEMSTATE/GETITEMTEXT 必须远程内存或走 WM_GETTEXT 编组）。

选中项读取修正记录：`LVM_GETNEXTITEM` 首次搜索 wParam 必须 = -1（`WPARAM(usize::MAX)`），
用 None(0) 会跳过第 0 行；设置选中态的 LVITEMW.mask 必须是 `LVIF_STATE`。

## ReBar UIA 缺席（已定性）

Win32 层 ReBar + 2 工具栏波段结构完整、位置正确（enum_children 实测 band 在 ReBar 客户区 233..259），
但 ReBar 自身窗口高 0（SetWindowPos 报成功却被系统立刻缩回——RBS_VARHEIGHT 与无图像列表的组合行为）。
UIA 因此把整棵 ReBar 子树裁剪掉。原版 ReBar 正常显示，差异在它的波段有真实按钮/图像列表。
**下一步**：给 ReBar 设置图像列表（RB_SETBARINFO 带 RBBIM_IMAGES + band 带 cxHeader），
波段按钮填充（需图标提取，assets/extracted/14/ 已备）。列头 SplitButton 化同因依赖此图标管线，一并排期。

## 图标管线 + comctl32 v6 manifest（2026-10-20 完成）

**根因发现**：ReBar 波段插入失败 / ImageList 32bpp 无效 / 控件无主题外观——
全部源于 exe 缺 comctl32 v6 application manifest（v5 行为差异）。
build.rs 用 embed-manifest（new_manifest 默认带 Common-Controls 6.0 依赖）嵌入。
assets/rc/qdir.rc/qdir.manifest 为独立参考（windres 可编译）。

**导航工具栏**（src/ui_native/toolbar.rs）：
- PNG 资源规整：assets/pngs/*.png（原版名是 ASCII 按 UTF-16LE 存储，提取时显示为乱码宽串）。
- ImageList_Create(ILC_COLOR32|ILC_MASK) + PNG→BGRA 顶朝下 DIB 注入（alpha 走掩码）。
- 按钮映射（FUN_00429bc4 实测 id，16 位截断后匹配 WM_COMMAND）：
  z6=0xA76C v6=0xA76D upF=0xA771 new=0xA772 view=0xA775 update=0xA770
  go=0xB379(BTNS_DROPDOWN) viewtree=0xA773。
- 接进 ReBar 波段 0（RB_INSERTBANDW，cyMinChild=54）；on_size 两轮布局：
  先拉 56 让 ReBar 排波段，再按 RB_GETBARHEIGHT 定型。

**UIA 验收结果（v11/final2 dump）**：
- d1 ReBarWindow32 进入控件树（此前缺席的根因即 v6 缺失）。
- d2 ToolBarWindow32 = 14（2 全局 + 每窗格 3），与原版 14 完全一致。
- 布局命令（1/2/3/4 窗格切换）：SplitterGroup.visible_panes + SWP_FRAMECHANGED 触发重排；
  *待复核*：各按钮 id 的精确布局语义（垂直/水平/1上2下等）需对照原版按钮 tooltip 与反编译确认。

## 布局模式引擎（本棒完成）

**证据链**：
- 原版 UIA dump（本地 control_tree 实测）给出布局栏 12 个按钮的精确
  tooltip：默认4格 / 3上1下2 / 3上2下1 / 2垂直 / 2水平 / 4垂直 / 4水平 /
  3垂直 / 3水平 / 3左1右2 / 3左2右1 / 1-Dir（均带「右击重置」）。
- FUN_00429bc4 反编译：导航栏 8 按钮注册顺序 = z6(0),v6(1),upF(2),
  new(3),view(4),update(5),go(6,BTNS_SPLIT),viewtree(7)；槽位顺序与
  UIA tooltip 顺序一致。
- **MSAA 探针（msaa_toolbar.rs）**：原版工具栏 IAccessible 子元素
  childId 均为 1..7 通用值、accName 空——原版未通过 MSAA 暴露按钮
  id/tooltip，此路不通（记录以免后人重蹈）。
- **外部 WM_COMMAND 探针（layout_probe.rs）**：原版对跨进程 PostMessage
  布局 id 无响应（窗格矩形逐字节不变），布局切换经由工具栏通知/下拉
  菜单内部路径，外部注入无法复核 id↔模式。

**实现**：`ui_native/splitter.rs::Layout`（12 模式枚举 + pane_count +
from_nav_id）+ `layout_rects`（12 模板递归切分，含 2×2/三格 L/T 形）+
`SplitterGroup::rects`（Default 模式按 SP0/1/2 万分比做嵌套 2×2，退化
回等分；其余模式走模板）。on_size 改用 rects 二维布局。测试抓到
pane_count 对 TwoVertical/TwoHorizontal 的 _ => 3 bug（已修）。

**已确证映射**（反编译 + UIA 顺序一致）：0xA76C=默认4格、0xA76D=2垂直、
0xA771=3上1下2、0xA772=3上2下1、0xA773=1-Dir。
**待复核**（PNG 命名+顺序推断）：0xA775=4垂直、0xA770=4水平、
0xB379(go 下拉)=其余模式。原版对这两个推断没有可执行的复核手段
（外部注入无效、MSAA 无名），只能等逐像素 UIA diff 阶段用原版真实
点击对照。

## 列头 SplitButton 对齐（本棒，部分达成）

**实测方法**：`examples/header_probe.rs`（GWL_STYLE + HDM_GETITEMCOUNT +
HDM_GETITEMW fmt 解码，跨进程只读安全）。

**已达成**：
- 列头窗口样式与原版逐位一致：`0x500000c2` =
  `HDS_BUTTONS|HDS_DRAGDROP|HDS_FULLROW|…`（原版与重写探针输出相同）。
- 列 `fmt` 加 `LVCFMT_SPLITBUTTON`（=HDF_SPLITBUTTON 0x1000000，v6 专属）：
  UIA 树里每 List 旁出现 1 个 SplitButton 节点；`LVCFMT_LEFT` 变体则
  列头完全不可见（对照组实验证明该位是暴露开关）。

**残留偏差**（记录，暂不修）：
- 原版：每表 4 个具名 `HeaderItem 0..3`（名称/修改日期/类型/大小）；
  重写版：每表 1 个无名 SplitButton。根因假设：原版 header 项带独立
  UIA provider 路径（可能需 owner-draw 或 HDITEM 逐项 SPLITBUTTON 状态，
  HDM_GETITEMW 跨进程读两版都返回 items=0，无法直接比对项格式）。
- 跨进程 HDM_GETITEMCOUNT 对两版都返回 0 → header 项格式无法用外部
  探针核对，只能同进程调试；列为后续项。

**测试注意**：真实 `Q-Dir.ini` 的 `QDir_Id` 每次原版启动自增（实例计数），
`config_tests::ini_byte_identical_roundtrip_on_real_file` 对该键只断言
"解析为正整数"；其余键（als/designe_mode/…）保持逐字节等价断言。

## 命令补齐进展 + 语言块定性（本棒）

**语言块定性**：IMG/CHS.bin 等熵 7.98 bit/byte = 压缩存储；FUN_004793f4
反编译确认为**算术编码器解码器**（range coder：low/range 状态机 +
256 概率表自适应，`DAT_00553f90/f88/f8c/f98/f9c` 为 low/code/range/
cumfreq 组）。逆它 = 重写原版自适应模型，工程量独立成棒，列为遗留。

**两条提取捷径实测失败（记录避免重蹈）**：
1. UIA ExpandCollapse 展开菜单读弹出 #32768：原版弹出层 MenuItem 的
   AutomationId 为空，拿不到 id→文案。
2. 跨进程 GetMenu/GetMenuStringW：原版主窗 GetMenu()=NULL——WTL 自绘
   菜单条不挂传统 HMENU。

**可行路线**：menus.json 里文本为字面量（非语言键）的条目直接给出
id→语义，本棒据此接入：
- 48504 = 文件>PowerShell（工作目录开 powershell.exe）。
- 45978..45990 = 视图缩放档 60/80/100/110/120/130/140/150/160/180/200
  （id 顺序与字面文本逐一核对），实现 `apply_ui_scale`：
  NONCLIENTMETRICS.lfMessageFont 按百分比 CreateFontIndirectW +
  EnumChildWindows 广播 WM_SETFONT。MainFrame 增 `ui_font/ui_scale`。
- 34064-34068 = 主题色档（#14F3FF (B)/#000000/#14F3FF/#999999/#matrix）
  待接（color.ini 主题棒）。45952/34183/34184 等仍待语言表。

**剩余命令 id 补齐的真正解锁件**：①算术解码器重写；或 ②等值推断+
原版行为对照（大目录逐 id 试验，成本高）。短期优先走已知的字面项与
id 段规律（57xxx=标准命令段等）。

## 交互核心：双击导航 / 页签切换 / 焦点跟随（本棒）

**通知路由**：ATL 容器 `pane_proc` 把 WM_NOTIFY 沿父链上送主框架；主框架
`on_notify` 按 `NMHDR.hwndFrom` 匹配 `panes[i].list/tab` 定位窗格：
- `NM_DBLCLK`（List）→ `list_dblclk`：选中目录→进入（history truncate+push、
  tab.path 更新、fill_pane 重灌）；选中文件→ ShellExecute open。
- `NM_CLICK`（List）→ 窗格焦点跟随（session.focus 更新，原版点击窗格即激活）。
- `TCN_SELCHANGE`（Tab）→ `tab_select`：读 TCM_GETCURSEL 更新 tabs_pos 并重灌。

**已验证**：`QDIR_TEST_DBLCLK_ROW=0` 启动实测——第 0 行目录被进入、历史压栈、
窗格重灌，无崩溃（端到端钩子复用 `dir_of_selection` 纯函数）。
`dir_of_selection` 5 个单元测试（含真实文件系统进出/根目录不导航）。

**语义注记**：`p == cur → 上级` 分支在当前列表（不含 ".." 行）下不可达，
保留待 ".." 行实现（原版列表有该行）；Backspace 返回上级是后续项。
**待复核**：TCN_SELCHANGE 只在真实多标签会话可测；start.qdr 是文件夹+
查询串混合格式（%xx% 编码），多标签解析列为遗留（与语言表无关）。

## 键盘导航（本棒）

控件子类化 `ui_native/subclass.rs`：用 comctl32 `SetWindowSubclass` 给每
窗格的 list 与 addr_edit 挂 WM_KEYDOWN 处理，refdata 编码 `pane<<1|role`。
- list Enter → `open_selection`（目录进入/文件默认程序打开）。
- list Backspace → `go_parent`（**history 回退优先**，符合原版后退语义；
  无历史时 parent 兜底）。闭合上棒「..」行不可达残留。
- addr_edit Enter → `addr_enter`（输入路径是目录则导航）。

导航逻辑抽成 `navigate_to/open_selection/go_parent`（list_dblclk 复用），
供鼠标/键盘共享。**端到端验证**（examples/key_probe.rs，跨进程 PostMessage
纯数值 WM_KEYDOWN + 读地址栏 WM_GETTEXT）：Enter 后地址栏由
`C:\Users\me` 变 `...\..agents`，Backspace 回退还原。240 测试全绿。

## 列排序（本棒）

`HDN_ITEMCLICKW`（header 发给父窗 ListView，被子类截获）→ `apply_column_sort`
→ 决策纯函数 `column_sort_click(当前键, 当前向, 列)`：列映射
0名称/1修改日期/2类型/3大小（原版列头顺序），同列翻转方向、异列升序。
更新 tab.sort_key/sort_dir 后 fill_pane 重排。

排序箭头：`paint_sort_glyphs` 在每次 fill_pane 后按当前排序对 4 列头
HDM_GETITEMW→清 HDF_SORTUP/DOWN→按需 HDM_SETITEMW 加箭头（comctl32 v6
原生绘制，与原版一致）。

**验证**：`column_sort_click` 3 单元测试（翻转/切列升序/列映射+越界）；
箭头绘制随 fill_pane 运行，冒烟（Enter 导航重灌）无崩溃无回归。
**限制**：header 点击是控件内生成通知，跨进程 PostMessage 无法可靠触发
（同布局按钮结论），端到端点击排序留待真实鼠标注入或同进程断言。

243 测试全绿。

## 主题着色：Filter2 → 行色（本棒）

`fill_pane` 填充时预计算每行前景色（`row_colors`，复用 `RuleSet::color_for`
——「第一条命中规则即决定颜色，-1 不再后找」，该纯决策 core 层已单测），
`NM_CUSTOMDRAW` 只读预算值：`CDDS_PREPAINT` 返回 `CDRF_NOTIFYITEMDRAW`，
`CDDS_ITEMPREPAINT` 命中行设 `clrText` + 返回 `CDRF_NEWFONT`。
另修：on_notify 未识别来源时透传 DefWindowProc（此前会吞掉 ReBar 通知）。

**实证生效**（QDIR_TEST_LOG_COLORS 钩子）：每窗格 119 行命中三类色值，
与 Q-Dir.ini 的 Filter2 规则精确对应：
- `008800` 绿 = `#DIR`（目录）
- `0000BB` 蓝 = `#Hidden`（隐藏/系统项）
- `BB8844` = 二进制/压缩扩展名规则

rgb_to_colorref 单测（0x00BBGGRR 序）。244 测试全绿。
**未做**：colorthema 34064-34068 档位（配色方案切换）与 #BG/#BG-A 交替
背景色；需要 UIA/截图级对照确认各档配色，列为后续。

## 配色系统（本棒，部分达成 + 边界说明）

**已交付**：
- 交替背景色 `#BG`/`#BG-A`：fill_pane 预算 `row_bg_colors`
  （`RuleSet::bg_variant(row)`，core 层已有决策+单测），custom-draw
  写 `clrTextBk`。原版 ini 中该两条规则为 `-1`（未启用），故默认无
  可见差异——与原件行为一致。
- colorthema 档位命令 34064..34068（菜单字面文本 → id 顺序核对）：
  切换 config/session.colorthema 并重灌四窗格（行色预算重算），
  状态链路端到端验证（0→4）。

**未达成（诚实记录）**：colorthema 五个档的**具体配色方案**未实现。
原因：该键仅存于 .qdr 的整数字段，渲染侧（每档的前景/背景/主题
色）无法从二进制取证（FUN_00429270/0x2fbe 是按钮显隐切换，与配色
无关）。当前档位切换是「状态正确、视觉暂无差异」的空壳，需要
截图级对照或原版调参实测才能补全。
**遗留**：截图级颜色 diff 工具（原始策略要求 UIA 机器 diff，颜色需
截图像素 diff，尚未实现）。

## 语言表/菜单文案：四条路径实测结论（本棒，定案）

**结论：语言表提取暂缓，改用 known_label 硬证据表 + 菜单文案快照兜底。**

四条路径逐一实测：
1. **IMG 语言块算术解码器**：CHS.bin 熵 7.98=压缩；FUN_004793f4
   反编译为 range coder（low/range/概率表状态机）。可重写但工作量大
   （自适应模型需完全对拍），未做。
2. **UIA ExpandCollapse 展开菜单**：原版弹出层 MenuItem 的
   AutomationId 为空，拿不到 id→文案。
3. **跨进程 GetMenu/GetMenuStringW**：原版主窗 GetMenu()=NULL
   （WTL 自绘菜单条不挂传统 HMENU），死路。
4. **SendInput 真实输入驱动 + UIA 读取消**：**部分成功**。
   - 屏幕固定坐标估算 + 鼠标点击：成功拿到 6/7 个顶级菜单共约 100 条
     中文文案（assets/menu_texts_zh.txt）。
   - 但逐项 diff 证明**位置序与 menus.json 不对应**（原版动态增删菜单
     项：Shell 上下文菜单、状态相关隐藏），故文案→id 绑定不可靠。
   - 改用 UIA GetClickablePoint（返回 false）/BoundingRectangle 中心
     点击：合成鼠标事件不被 WTL 菜单条接收（非客户区命中测试需要真实
     输入轨迹），后续窗口状态变化后连固定坐标也失效。
   - 递归子菜单 walk_menu 已实现但因上述输入问题无法全量跑。

**已落地（替代方案）**：
- `assets/menu_texts_zh.txt`：约 100 条原版中文菜单文案快照（人工
  对照基准）。
- `commands::known_label(id)`：只收**已实证**的 id→文案（各棒验证过
  的 30 项），「未接命令」日志现在带中文提示。

**证据文件易变性（重要运维知识）**：原版每次运行会改写
`D:\Q-Dir_Portable_x64\Q-Dir\` 下的真实配置——QDir_Id（实例计数，
退出归零）、Focus（最后活动窗格，实测 P1/P3）、ColsAndSort1（列交互
串，2341/2342 字符浮动）。config_tests 对这三个键已改为量级/合法性
断言，其余键保持逐字节等价。**今后调原版后若测试失败，先查这三项。**

## 每窗格命令按钮排接入（id=base+20..29）

- 原版每窗格 10 个命令按钮（UIA tooltip 顺序）：到上一个选中项 / 到下一个
  选中项 / 运行 / 通过电子邮件发送 / 删除 / 剪切 / 复制 / 粘贴 /
  过滤器或高亮 / 快速链接菜单。create_pane 早已按 `1000+100*pane+20+bi`
  创建，但全部落在「未接命令」；本棒接入 dispatch。
- `commands::pane_button_id(id)` 解码 (窗格, 序号)，纯函数，3 个单元测试
  （含与 cmd::*/layout::* 的无重叠回归）。
- 语义映射：0/1 → `move_selection_to_next`（LVM_GETNEXTITEM 沿 LVNI_SELECTED
  ± 方向移焦点，ENSUREVISIBLE，选中集合不变）；2 → shell "open" 首选项；
  3 → 新增 `shell::send_by_mail`（verb "mail"，MAPI 外壳扩展）；4..7 →
  复用已验证的 delete/cut/copy/paste，但**作用域=按钮所在窗格**而非活动
  焦点窗格（`selected_paths(bp)`/`pane_path(bp)`）；8 → 聚焦该窗格过滤框；
  9 → 占位（聚焦列表 + TEST 日志，书签弹层待 QuickLink 数据解析）。
- 端到端验证（QDIR_TEST_SELECT_ROW=1 + PostMessage 数值 WM_COMMAND）：
  id=1026 → 剪贴板出现选中路径；1021/1028 → TEST 日志窗格1 序号1/8。
- 诚实声明：**原版这些按钮的真实命令 id 从未反编译确认**；id 方案是本
  重写版自定约定，序号→操作按 tooltip 字面直译，*待复核*。外部
  PostMessage 对原版注入无效（早前已验证），故按钮行为只能待 UI 内
  点击级对照（截图/行为 diff 前置件）最终定案。

### 按钮命令的 WM_COMMAND 转发（实测补记）

- 命令按钮是 content（ATL 容器类）的子窗：不转发则 BN_CLICKED 止于
  content 的 DefWindowProc。`pane_proc` 现把 **id 命中 pane_button_id 的
  WM_COMMAND** 沿父链上送（button→content→container→主窗）。
- 曾试过无条件转发 WM_COMMAND：会把 addr/filter Edit 的 EN_* 通知（id
  base+1/3）涌进未接日志（实测 id=1001 每窗格两条）——改为按按钮 id
  过滤后噪声归零，端到端复验通过（id=1026 → 剪贴板选中路径；
  1021/1028 → TEST 日志序号正确）。

## 视图/收藏夹菜单项接线（德语模板 #216/#217 双语硬证据批）

- 资源里的德语菜单模板 #216/#217 给出**字面文本 ↔ id** 硬证据（与中文
  UIA dump 互证，无位置对齐猜测）：
  - `32833..32836` `# 1..# 4` → 活动窗口 N：`set_active_pane`（session.focus）。
  - `33931..33934` Große/Kleine Symbole / Liste / Details → 视图模式：
    LVM_SETVIEW（33933→LIST、33934→DETAILS 字面对应；33931/33932→
    ICON/SMALLICON 按语义推断，*待复核*——列表当前无大图标图像列表）。
  - `32864` &Linien anzeigen → 网格线：LVS_EX_GRIDLINES 读-改-写切换。
  - `33938/33939` Spaltenbreite an Text anpassen (mit Spaltenkopf) →
    列宽自适应：LVSCW_AUTOSIZE_USEHEADER / LVSCW_AUTOSIZE。
  - `32919` Zu Favoriten hinzufügen（中文 UIA「添加到收藏夹...」互证）→
    新增 `shell::create_shortcut`（IShellLinkW + IPersistFile::Save）把
    活动窗格首选项写成 Favoriten 目录下 .lnk（原版 ini 键
    `Ordner=.\Favoriten\` 相对 exe 解析）。
- 端到端实测：WM_COMMAND 注入 32835/33933/33934/32864/33939/32919 全部
  TEST 日志命中；32919 产出的 .lnk 经 `file(1)` 验证为合法 shell link
  （LocalBasePath 指向选中目录），并有 `create_shortcut_writes_valid_lnk`
  单元测试（HEADER 魔数 + 大小写无关路径查找，shell 会归一大小写）。
- 已知偏差：.lnk 命名规则（前缀/编号）无原版样本（用户 Favoriten 目录
  为空），取目标名直译，*待样本复核*；收藏夹 Quick-Link 弹层（bi=9）仍占位。

## 快速链接菜单（命令按钮 bi=9）真实现

- 此前 bi=9 是占位（聚焦列表 + 留痕）。本棒实现为真功能：
  `MainFrame::show_quick_link_menu` 扫描 `Link=.\Favoriten\Quick-Link\`
  （原版 ini [Favoriten] 键，硬证据）里的 .lnk → 逐个 `read_shortcut_target`
  读目标 → CreatePopupMenu/AppendMenuW（项 id 段 0x5A00+i）→
  TrackPopupMenu(TPM_RETURNCMD|TPM_LEFTALIGN|TPM_BOTTOMALIGN) 在按钮下沿
  弹出 → 选中项 id 反解序号 → `navigate_to(窗格, 目标)`。
- `shell::read_shortcut_target`：CoCreateInstance(ShellLink) →
  IPersistFile::Load(STGM_READ) → IShellLinkW::GetPath(SLGP_RAWPATH)，
  不触发解析对话框。`create_shortcut` ↔ `read_shortcut_target` 有
  `shortcut_write_then_read_roundtrip` 往返单元测试（大小写归一用同常量路径）。
- 用 TPM_RETURNCMD 同步取值而非 WM_COMMAND 回调，避开 dispatch 重入。
- 外部验证：Quick-Link 目录放 2 个 .lnk → WM_COMMAND 1029 →
  TEST「发现 2 项」日志；弹层是模态阻塞的，探针在弹层阶段结束（预期）。
  空目录 → 「发现 0 项」直接返回，不弹层。
- 相对路径解析统一为 `commands::resolve_relative`（[Favoriten] Ordner/Link
  都按 exe 目录解析；32919 收藏夹写入也改用它）。
- 已知偏差：原版 tooltip 第二行「鼠标右击选择在哪个窗口中打开」= 右键
  选目标窗格，本实现先固定导航按钮所在窗格；子文件夹嵌套未做，待样本复核。

## 附录 A：命令行为总表（Ghidra 反编译确证版，2026-10-21/22 会话产出）

> 来源：`D:\qdir_reverse\decompiled\all_functions.c`（6.4MB 全量伪 C）。
> 主分发器 = `FUN_00475950`（主窗 WM_COMMAND 总线）；子路由：
> `FUN_00470c30`（79KB catch-all，122 分支）、`FUN_00463cf4`（文案串族）、
> `FUN_00474a34`（语言键槽族）、`FUN_0046bd8c`（布局显隐）、
> `FUN_0046dc10`（树显隐/树着色）、`FUN_00467268`（0x8019–0x802b 颜色/文件夹开关）、
> `FUN_0045b114`（0x80c0–0x80cb 列显隐）、`FUN_0041fe84/0049747c/00497c64`（导航工具栏）。
> 本表是 PRODUCT_SPEC §3 功能清单的证据来源；id=菜单模板 128/216/217 的
> WM_COMMAND 低位 id；「确证」= 处理器行为已读源码；*待复核* = 机制清楚、
> 业务命名靠菜单文案/上下文推断。

### A.1 文件菜单（语言键 10001–10006）——**推翻旧推断**

| id | 十进制 | 旧推断（错） | 反编译确证行为 | 证据 |
|---|---|---|---|---|
| 0xE100 | 57600 | 新建文件夹 | **重复窗口**：保存状态到 `<exedir>\start2.qdr` → 以该文件为参数启动新实例 | FUN_00462814：`lstrlenW(L"\start2.qdr")` + FUN_0045cf90(保存) + FUN_0045a1e0(ShellExecute 自启动) |
| 0xE101 | 57601 | 复制 | **打开 Q-Dir 状态(O)**：GetOpenFileNameW 过滤 `Q-Dir File(*.qdr)` → FUN_0046cc14(载入) | FUN_00470a88 全文 |
| 0xE103 | 57603 | 剪切 | **保存当前状态(S)**：命名后 FUN_00468b3c 写出 .qdr | FUN_0046ab8c（0xe103 与 0x8034 共路由） |
| 0x8034 | 32820 | 粘贴 | **在当前桌面项目保存状态(L)**：同上但路径=SHGetSpecialFolderLocation(CSIDL_DESKTOP) | FUN_0046ab8c 的 `sVar9==-0x7fcc` 分支 |
| 0xE141 | 57665 | — | **退出**：PostMessage(WM_CLOSE)（早前已证实，再次互证） | FUN_00475950 |
| 0xBD75 | 48501 | — | **以管理员身份重新启动**：新实例 ShellExecuteExW verb=`"runas"`（.rdata 0x528100） | FUN_004626b8(-0x428b) + FUN_004764a0(1,…)+0x528100 |
| 0xBD77 | 48503 | — | **重新启动**：DAT_00550fd0=2（重启旗标）→ 连发 0xe100(存状态)+0xe141(退出)；旗标=2 时新进程用 explorer.exe 拉起自身（绕开启动器双进程） | FUN_004626b8(-0x4289) + FUN_0045a1e0 |

> 结论：**原版文件菜单没有剪贴板项**。复制/剪切/粘贴只存在于 Shell 上下文
> 菜单（模板 #14 族）；窗格命令按钮排（bi=4..7 的剪贴板语义）保留我们自己的
> tooltip 直译约定，*待复核* 状态不变。

### A.2 工具类（文件菜单中段）

| id | 十进制 | 行为 | 证据 |
|---|---|---|---|
| 0xB335 | 45877 | **命令行**：活动窗格路径起 `cmd.exe`（ShellExecute 带 cd） | FUN_004cc524(list,0,…)：串 `"cmd.exe"` |
| 0xBD78 | 48504 | **PowerShell**：同一函数 param≠0 分支 → `"powershell.exe"` | FUN_004cc524(list,1,…)：串 `"powershell.exe"` |
| 0xB358 | 45912 | **运行对话框**：CoCreateInstance(Shell.Application{13709620-C279-11CE-A49E-444553540000}) → IShellDispatch vtable 槽 **0x80 = FileRun** | 槽位表 windows-rs IShellDispatch_Vtbl 实测 + 本机注册表 CLSID 名 |
| 0xB359 | 45913 | **关机对话框**：同对象 → 槽 **0xA0 = ShutdownWindows** | 同上；语言键 205/206=Windows 标准「运行/关机」 |
| 0x8030 | 32816 | **导出(X)**：收集活动窗格列表 → TXT/CSV/HTML/XLS 导出（串 `List`/`Export TXT,CSV,HTML,XLS`/`.txt`） | FUN_00463cf4(0x8030)+FUN_00451318 |
| 0xB325 | 45861 | 活动窗格路径取值后释放——疑似 **复制路径**类 | FUN_00470c30 0xb325 分支；*待复核* |

### A.3 树显隐与着色（视图菜单 54000 + 导航 viewtree 按钮）

- `FUN_0046dc10` 统一处理，写配置键 `tree_mode`：
  - 0x806c(32876,54001)=tree_mode**1**（双列树：树+列表并排，全窗格重排）
  - 0x806d(32877,54002)=tree_mode**2**（树覆盖模式）
  - 0x806b(32875,54003)=tree_mode**0**（关树）
  - 0x806a(32874,54006 系)=tree_mode **+1 循环**
- 导航栏 `viewtree.png`(0xA773)：**非布局按钮**——按当前值改发 0x806c/0x806b 即树开/关切换（FUN_0041fe84 转换表确证）。旧注释「1-Dir(一个窗口)」作废。
- 0x413/0x414/0x415（1043–1045，54006 组）= `useTreeColor` = 0/1/2 **树着色三档**。

### A.4 布局切换（菜单 52001–52012 → ids 33943–33954 = FUN_0046bd8c）

可见性元组（`FUN_00455b3c(1×4)` 参数序=窗格 1..4 显隐布尔；e768 列=三组面板树/预览开关）：

| id | 十进制 | 窗格显隐 | 面板 |
|---|---|---|---|
| 33943 | 0x8497 | (1,1,1,1) | 双 a228 重排（2×2 基准） |
| 33944 | 0x8498 | (1,0,1,1) | — |
| 33945 | 0x8499 | (1,1,1,0) | — |
| 33946 | 0x849A | (1,1,0,0) | — |
| 33947 | 0x849B | *链式穿透至默认段* | *待复核*（bVar 标志组合，疑带面板变体） |
| 33948 | 0x849C | (1,1,1,1) | e768:0x7ad8=1,0x7c28=1（开树/预览） |
| 33949 | 0x849D | *待复核*（同 k5/k7 组内链式） | — |
| 33950 | 0x849E | (1,1,1,0) | e768:0x7ad8=1,0x7c28=1 |
| 33951 | 0x849F | *待复核* | — |
| 33952 | 0x84A0 | (1,0,1,1) | e768:0x7ad8=1,0x7b80=1,0x7c28=0 |
| 33953 | 0x84A1 | (1,1,1,0) | e768:0x7ad8=1,0x7b80=0 |
| 33954 | 0x84A2 | (1,0,0,0) | **单窗格（=旧猜的 1-Dir）** |

- 导航按钮 `z6.png`(0xA76C)/`v6.png`(0xA76D) = **布局索引 −1/+1 循环**（FUN_0049747c→FUN_00497254(idx±1)，边界回绕），**不是**「4-Dir/2-Dir」具体档；`upf.png`(0xA771)=弹出布局列表菜单（CreatePopupMenu 枚举+TrackPopupMenu）；`new.png`(0xA772)=FUN_00481604 新窗类*待复核*；`update.png`(0xA770)=刷新重绘；`view.png`(0xA775)=向窗格内容窗发 0xd1f；`go.png`(0xB379)=向窗格内容窗发 0xd20（下拉）。

### A.5 视图模式（33931–33934 = 0x848B–0x848E）

FUN_004d0370：按 id 给活动窗格内容窗发 **0x7029/0x702A/0x702B/0x702C**（私有消息，疑似转发 LVM_SETVIEW 0/2/3/1——与德语模板 Große/Kleine/Liste/Details 顺序互证）。旧「LVM_SETVIEW 直映射」保留为近似实现；确切转发语义 *待复核*（需读 0xd0370 全函数）。

### A.6 关于/帮助（Help 菜单）

- 0xE140(57664)=**关于**：参数块 lang 0x3a → 关于对话框（FUN_00463cf4 首分支）。
- 0xE146(57670)=**在线帮助**：`FUN_0049fd78`（串 `http...-help#`）。
- 0xE147(57671)=**按语言重启**：13 组「语言名↔locale」表（German/de-DE … Arabic/ar-SA），命中项写入语言键 DAT_00551be8。
- 0x800C/0x800D/0x800E/0x800F(32780–32783,60004–60007)=主页/?Freeware/paypal 捐赠/软件链接（内联逐字节 URL 串重建：`http://www.softwareok.de|om/` 双缓冲按语言选）。0x800F=mailto 反馈 `?Kontakt&Betreff=`。0x852C=FAQ 页。0x8593=Windows/Info 页。0x8031/0x8032=复制信息/诊断包（GetTempPath `.zip`）。

### A.7 语言键槽族（FUN_00474a34，id∈0xB798–0xBB80）

机制：每个 id = 一个**已存状态槽**（表 param+0x7C90）；单击=载入该槽（FUN_0046cc14），Ctrl=复制路径，Shift=另存到槽，Del=删除。对应 Q-Dir「桌面/布局槽」功能。*业务命名待复核*（槽的 UI 入口在哪个菜单未逐条对照）。

> **勘验增量（2026-10-24）**：主 WM_COMMAND 分发器确认槽族入口条件
> `(ushort)(id + 0x4868) < 0x3e9` —— 无符号回绕等价于 **id∈[0xB798,0xBB80]**
> （0x10000−0x4868=0xB798，与本表记载一致），即 1000 个连续 WM_COMMAND id
> 直接进 FUN_00474a34（AF:86814）。内部再证：槽表键查 `FUN_004ee518(
> param+0x7C90, id, …)`；Ctrl（GetKeyState(0x11)<0）→ 路径复制到
> param+0xC00 + SetTimer(0x2711, 300ms)；Shift（0x10<0）→ FUN_004a037c/
> FUN_004a0414 另存链；默认=载入。**1000 个 id 的菜单/弹层构建源在 AF 仍无
> 静态命中**（疑 TrackPopupMenu 动态 id 或外部宿主），入口位置维持 *待复核*。

### A.8 开关型设置项（catch-all 直接写配置键——行为=改 ini+重绘）

确证键映射（读 DAT + FUN_004554a0(写键) 即得）：audio_click=0x84F5(34013)、select_last_if_navigate_up=0x84F4、refresh_keep_sel=0xB339(45881)、show_tabs=0x8558/0x8559(34136/34137)、tree_auto_sel=0xB31E、tree_fullrowselect=0xB250、autocomplete=0xB36F、autocomplete_url=0xB370、address_ico=0xB375/6、adress_sw_driver=0xB336、adress_tree=0xB325 组、useColor/useColor2=0x84F4 组、folders=0x8019–0x802b 段（FUN_00467268）。列显隐 0x80C0–0x80CB（FUN_0045b114，写 `Als` 位）。

> **实现指引**：本表按「行为=改配置键+重绘」逐项落到自绘设置，不需要原版
> 文案；文案由我们自定（R1）。旧 commands.rs 的 cmd::NEW_FOLDER/COPY/CUT/
> PASTE 常量语义作废，落地 P1 时按 A.1 重命名。

### A.9 Spatlen_* 视图状态 blob（ini [Column_OS_10.0_PloderN]，2026-10-24 字节取证）

证据源：`D:\Q-Dir_Portable_x64\Q-Dir\Q-Dir.ini`（UTF-16LE，sha 见原版目录）。
该段仅有 `Spatlen_::{20D04FE0-…}` 与 `Spatlen_329` 两种键，值=**%XX 百分号
转义字节串**：

1. 解码后 798 字节 = **772 净载荷 + 26 字节 ASCII 尾标**
   `|CODEMODE1|-1905896973|772`（`Spatlen_329` 为 797=772+25，校验值不同）。
   尾标=`|CODEMODE<版本>|<i32 校验>|<净载荷长度>`；校验算法未确证 *待复核*。
2. 净载荷是 **shell 视图状态属性袋**："1SPS" 魔数 + FMTID
   d5cdd505-2e9c-101b-9397-08002b2cf9ae（Explorer IShellView::SaveViewState
   的标准序列化）。**条目不做 4 字节对齐**（逐键紧排、各键起点奇偶不一）
   ——解析必须锚定键名，不能游标步进。
3. 已确证标量键（键名  → u32 VT → u32 值；VT_BOOL=11 取低 16 位有符号）：
   `GroupByKey:FMTID`(VT_BSTR="{B725F130-47EF-101A-A5F1-02608C9EEBAC}")、
   `GroupByDirection`=1、`Sort`(VT_STREAM="prop4294967295")、`GroupView`=-1(true)、
   `ViewMode`（紧邻二进制块，字节语义未闭合 *待复核*）、`IconSize`=16、
   `ColInfo`(VT_STREAM)、`GroupByKey:PID`=4、`FFlags`=0x40200005=**1075838981**、
   `LogicalViewMode`=1。
4. 交叉验证：IconSize=16 / FFlags=1075838981 与 L-Dir core 默认值一致。

> **L-Dir 落地（P2.6）**：`shell::viewstate` 锚定键名解码 →
> `core::ColumnViewState` 投影（icon_size/folder_flags/group/logical_view_mode/
> group_by_dir），`load_config_bytes` 导入 [Column_OS_10.0_PloderN] 各段，
> `MainFrame::new` 应用到窗格（view_mode 不覆盖——.qdr 优先 *待复核*）。
> 本程序保存 ini 不写出 Spatlen（R2 自名格式）。Sort/ColInfo/键序列表等
> 复杂键未建模 *待复核*。
