# L-Dir 产品规格与开发计划（2026-10-21 战略转向版）

> 工作名 L-Dir（待定稿，见 §6 决策项）。定位：Q-Dir 同类四格文件管理器，
> Rust + 原生 Win32。本文是转向后的**总纲**；历史逆向证据收编于
> NATIVE_SPEC.md（参考档案，不再是验收目标）。

## 0. 战略（用户定案，2026-10-21）

不做 1:1 复刻。**三层切分**：

| 层 | 态度 | 说明 |
|---|---|---|
| 布局体验 | **必须对标 Q-Dir** | 四格 + 每格标签页 + 导航布局按钮排 + 地址/过滤行 + 命令按钮排 + 状态栏。这是 Q-Dir 迭代多年沉淀的空间模型，用户肌肉记忆所在。 |
| 交互行为 | **必须对标 Q-Dir** | 行为语义按 NATIVE_SPEC 命令行为总表（逆向确证版）逐项实现：布局即窗格显隐、状态打开/保存走 .qdr、收藏夹=.lnk 目录 + Quick-Link 弹层、视图=大小图标/列表/详情、缩放重排非只换字体等。快捷键尽量对齐原版。 |
| 界面外观 | **自主设计** | 自绘/自选：菜单文案与结构、对话框模板、图标、配色、主题、字体。免除资源级一致的全部成本。 |

**价值来源**：原版不是被抄的，是被「懂的人」重做的——行为表来自 Ghidra
全量反编译（FUN_00475950/FUN_00470c30 等 122+ 分支，见 NATIVE_SPEC 附录）。

## 1. 规则（开发规矩，违反即返工）

- **R1 零原版资产入产物**。产品二进制（qdir-rs.exe）不得内嵌任何源自
  Q-Dir_x64.exe 的字节：PNG（toolbar.rs 现状，P0 处理）、菜单/对话框模板
  字节（res.rs→app.rs 菜单 128 现状，P0 处理）、原版文案、语言块、
  图标、声音。`assets/extracted/`、`assets/pngs/`、`assets/menu_texts_zh.txt`
  保留为**只读参考**，不得 include_bytes 进 src。
- **R2 兼容是特性不是枷锁**。配置层保留：可**导入**原版
  Q-Dir.ini / start.qdr / Favoriten 目录（现有字节级解析+往返测试就是
  这个能力的证据）；但产品默认写自己的文件名/位置（见 §6）。
  解析格式按原版取证实现，写出不要求逐字节等价原版。
- **R3 功能范围以行为表为准**。要做什么/不做什么，从 NATIVE_SPEC 附录
  的「命令行为总表」勾选；增删功能是产品决策，不再以「与原版一致」
  为验收依据。
- **R4 验收=功能自证**。每棒：`cargo build` + `cargo test` 全绿为底线；
  功能正确性用进程内 TEST 钩子（QDIR_TEST_*）+ 端到端探针 + UIA 结构
  检查（control_tree 从「与原版 diff」改为「结构健全性检查」）。
  pixdiff 降级为开发期工具，不再是 gate。
- **R5 诚实记录**。本文件记决策与进度；每个功能标注：已实现+证据 /
  部分+缺口 / 待做。推断标 *待复核* 的旧规矩保留。
- **R6 环境纪律不变**：构建前 `taskkill //IM qdir-rs.exe //F`；Git Bash
  /tmp 不对 py 可见；PowerShell 用 -File；原版运行会漂移 QDir_Id/
  Focus/ColsAndSort1 三个键（仅在 compat 测试中受影响时检查）。

## 2. 现有资产盘点（留/改/删）

| 资产 | 处置 | 理由 |
|---|---|---|
| src/core.rs 全层（Config/SessionState/PaneState/TabState/Qdr/分割/排序/高亮规则模型） | **留** | 纯逻辑、平台无关、测试全绿；就是产品数据模型。 |
| src/config.rs ini/qdr UTF-16LE 解析-往返 | **留（改目标）** | R2 兼容导入的地基；字节等价断言降级为「可导入」证据。 |
| src/services/{favorites,export,i18n} | **留** | 目录枚举/lnk 读写/五格式导出与原版无绑定。 |
| src/shell 全层（剪贴板/文件操作/图标/PIDL/特殊文件夹/托盘/拖放） | **留** | 与 1:1 无关的产品地基，质量已被端到端验证反复背书。 |
| src/ui_native 结构层（ReBar/状态栏/四窗格/分割器/ListView custom-draw/子类化/字体缩放） | **留** | 「布局体验对标」的实现本身。 |
| src/ui_native/res.rs + app.rs 加载菜单 128 | **删/替换（P0）** | 原版字节违反 R1；换自建菜单（CreateMenuW，自有文案与 id）。 |
| src/ui_native/toolbar.rs 12 个 include_bytes PNG | **替换（P0）** | 原版位图；功能语义（8 按钮：4/2/3上2下/3上2下?/视图/更新/下拉/树）保留，图自己画。 |
| 标题 "Q-Dir"、状态栏 "12.71"、w!("Q-Dir") 各处 | **改（P0）** | 品牌字段，统一从一处常量改。 |
| assets/extracted、assets/pngs、assets/rc、menu_texts_zh.txt | **归档** | 参考只读；config_tests 的夹具可继续从 assets 读。 |
| NATIVE_SPEC.md | **收编附录** | 行为总表 = 功能需求清单来源。 |
| examples/pixdiff、control_tree diff 用法 | **降级** | control_tree 改结构检查；pixdiff 留作自查工具不再 gate。 |
| --egui 对照路线 | **删（择机）** | 1:1 时代的实验，已无价值。 |
| 语言表解码（FUN_004793f4 重写） | **取消** | 自作文案，不需要原版文案。 |

## 3. 功能清单（从行为表勾选，= R3 的范围表）

P1 必须（产品可用性），按行为表证据实现；P2 体验完整；P3 长尾。

**P1**
1. 四格 + 每格标签页 + 窗格内地址/过滤/命令按钮排（骨架已有；按钮语义已确证）
2. ✅ 布局（2026-10-22）：导航按钮按附录 A.4 确证实现——z6/v6=12 档索引
   −1/+1 循环（Layout::ALL）、upf/go=12 档弹出菜单（build_layout_menu，
   TrackPopupMenu TPM_RETURNCMD，id 段 30090+）、update=全窗格重灌刷新、
   viewtree=tree_mode 切换、view=转发留痕（0xd1f 待内容窗）。
   冒烟实测：循环 3 步 2×2→单格→2×2→左右、tree_mode=1、30099→四格(4 列)，
   窗口存活。剩余：视图菜单 52001–52012 显隐表（附录 A.4 已解 8/12，
   k=4/6/8 链式穿透段）属原版内部布局序，我们的 Layout::ALL 序即自定序。
3. ✅ 树控件（2026-10-22）：`ui_native/tree.rs` SysTreeView32 接入——tree_mode
   三态（0 关 / 1 双列 / 2 覆盖，附录 A.3 `FUN_0046dc10` 确证；`core::TreeMode`
   取代旧 bool，.qdr/ini 读写兼容 tree_mode=0/1/2）；盘符根并列 + 懒加载
   子目录（TVE_EXPAND 首次展开枚举）+ 根行复选框（TreeN 位，*跟随子目录
   行为待复核*）；列表导航→树自动展开祖先链并选中（reveal，tree_auto_
   expand_from_list 语义），树选中→列表导航（TVN_SELCHANGED→navigate_to，
   busy 抑制回环）。viewtree 按钮=开关切换（toggled），视图菜单新增
   双列/覆盖/关闭/循环四档（原 54001/54002/54003/54006 系）。
   **顺带补上窗格内部布局 pass**（layout_pane_children：addr/工具条排/
   页签/命令按钮两行五列/过滤行/树|列双列或覆盖切分/状态栏——此前子控件
   从未被定位，全挤在 (0,0,10,10)；树几何暴露了这个空档）。
   实测：30030→4 树可见（双列 60×61px 于 800×600 窗，宽=250‰ 树分割），
   30031→覆盖（175×53 浮层），30033 循环/30029 开关全绿；
   QDIR_TEST_TREE_SELECT 钩子端到端：展开 C:\ → 懒加载子目录 →
   选中 C:\$RECYCLE.BIN → 窗格导航成功；启动 .qdr tree_mode=1 时
   reveal 定位当前目录 0 失败。树图标=系统小图标列表（SHGetFileInfoW
   SHGFI_SYSICONINDEX 缓存句柄，TVM_SETIMAGELIST）。
   263 lib + 11 集成全绿。
4. 视图：窗格级 0x7029–0x702c（大图标/小图标/列表/详情，槽位确证）；
   列宽自适应 33938/33939（已有）；网格线 32864（已有）
4. 状态：打开 .qdr（GetOpenFileName→FUN_0046cc14 语义）、保存当前状态
   （GetSaveFileName→FUN_0045cf90）、重复窗口（新建实例加载 start2.qdr）、
   保存到桌面（.qdr 进桌面）
5. 收藏夹：32919 添加到收藏夹（已有）、32779 管理、Quick-Link 弹层（已有）
6. 工具：45877 命令行(cmd)/48504 PowerShell（确证同一函数双分支）、
   45912 运行对话框、45913 关机对话框（IShellDispatch 槽 0x80/0xa0 确证）、
   刷新 34037（确证=重读当前目录，refresh_keep_sel 开关确证）
7. 树显隐切换（0x806b/c 确证）、缩放档 60–200%（重排字体）、主题切换
   （33943 系…按自主配色实现，行为=改窗格底色规则）

**P2 部分已完成**
- ✅ 列表导出接 UI（2026-10-22）：文件菜单「导出列表(&E)…」(30009)，
  GetSaveFileNameW 四档 filter（TXT/CSV/HTML/XLS，默认 .txt），扩展名→格式
  `export::format_from_ext`（纯函数单测）；导出**全窗格当前列表**（原版
  0x8030 行为：非选中项），复用 sort_items 排序；QDIR_TEST_EXPORT 钩子
  冒烟 146 项 CSV 落盘成功。
- ✅ 窗格快速过滤（2026-10-22）：过滤框 EN_CHANGE（pane_base+3，通知码
  768）经 pane_proc 上送→MainFrame::on_command 读发件人句柄文本→
  `pane_filter[idx]` 存储→fill_pane 应用「大小写无关包含」筛名
  （`core::filter_by_query` 纯函数单测；*原版 EN_CHANGE 处理器未反编译
  确证，语义按 Explorer 搜索框惯例，待复核*）。QDIR_TEST_FILTER 钩子
  实测 146→3 行（`.log` 命中当前目录 3 个日志）、清空恢复原状。
  探针通道备注：跨进程手工投递 EN_CHANGE 时 GetWindowTextW 读空
  （lParam 句柄跨进程无效），与原版拒绝外部 WM_COMMAND 同类限制，
  行为验证一律走进程内钩子。
- ✅ 打印 + 查找(F3)（2026-10-22）：文件菜单「打印(&R)」(30010)=选中文件
  shell `print` 动词（`shell::print_paths`，原菜单项确证存在、命令 id 与
  具体行为未反编译确证*待复核*，冒烟走无选中留痕分支）；「查找(&F)」(30011)
  与消息泵加速键表 F3（CreateAcceleratorTableW+TranslateAcceleratorW）=
  聚焦活动窗格过滤框，键入即接上面的快速过滤。实测：F3 后 GetGUIThreadInfo
  焦点句柄=窗格1 过滤 Edit（id 1003）。
- ✅ 预览面板 Vorschau 接原生 UI（2026-10-22）：每窗格 content 内
  Static(SS_BITMAP)+只读多行 Edit 双控件；`toggle_preview` 写会话+重排
  （列表让出右侧列，宽=(1-preview_pos) 钳 15%..50%）；NM_CLICK/
  LVN_ITEMCHANGED/fill_pane 三路刷新 `update_preview_pane`，数据走
  panes::preview（Image→CreateDIBSection BGRA 合成白底→STM_SETIMAGE，
  Text/Binary→Edit 文本）；view 按钮(0xA775 原版 0xd1f)按 Vorschau
  开关解释（附录 A.4 字面留档*待复核*）；查看菜单「预览面板(&P)」
  30034。QDIR_TEST_PREVIEW[=img|_DIR=覆盖目录] 钩子实测：文本分支
  mode=0（.bash_history 前导 hex）、图片分支 mode=1（ThemeA img20.jpg，
  Static 显 Edit 隐）、列表宽 390→289 挤压正确、30034 开→关→开控件
  显隐正确。附带修复：`apply_session_to_doc` 的 8 个顶层键
  set_in_place→set——原语义在**新建 .qdr** 时静默丢键（Vorschau 往返
  单测暴露；旧文件原位替换语义不变）。Vorschau/VorschauPos 往返单测。
- ✅ 标签行为开关（2026-10-22）：`core::tab_click_action` 纯函数决策
  （TabConfig.close_by_mbt_clk / close_by_lbt_dblclk，原版配置键分支确证；
  单测）。子类化扩展第三角色 ROLE_TAB（refdata 编码改 `pane<<2|role`）：
  SysTabControl32 上挂 WM_MBUTTONUP=关标签（TCM_HITTEST 定位→
  core::close_tab 拒绝最后一个=原版行为→重建页签+重灌）、
  WM_LBUTTONDBLCLK=开关关时吞事件防标签跳变；show_tooltips=true 建窗时
  加 TCS_TOOLTIPS（GWL_STYLE 读回确证 0x4000 位）。悬停标题气泡由 comctl
  内建（页签文本=标签标题）。QDIR_TEST_TAB_CLOSE 钩子双分支实测：
  =1 中键 2→1 关闭、=2 双击默认开关=吞事件保持 2。
- ✅ 主题色接 UI（2026-10-22）：`menu::theme_palette` 五档自设计配色
  （R3——原版字面 `#14F3FF (B)/#000000/#14F3FF/#999999/#matrix` 只确证
  档位数量与字面值，渲染语义无字节取证；我们把字面十六进制值落为
  窗格列表/树底色+前景，(B) 解读为深色底变体，matrix=黑底绿字）。
  `MainFrame::apply_theme_palette`：LVM/TVM SETBKCOLOR/SETTEXTCOLOR +
  重灌四窗格 + RedrawWindow(ALLCHILDREN)；档 0 用 CLR_DEFAULT 复位系统色。
  Filter2 命中行的前景色优先级不变（custom-draw 按预算色写回）。
  实测：LVM_GETBKCOLOR 四档读回 001b1414/00fff314/00999999/00081800
  （BGR 序与 RGB 设计值一致），复位后控件回系统默认。单测 1 个。

- 高亮/过滤规则编辑对话框（自绘模板）、
标签行为开关（双击关/中键关/提示气泡——分支确证）、打印、查找、
颜色主题档位（自设计）、状态栏 5 段信息完善
**P3** Shell 扩展菜单宿主、Everything/MS-Search 集成、多语言 UI（自有文案表）、
便携/安装版差异、托盘行为、开机恢复、拖放精修

## 4. 开发计划（阶段制，每阶段全绿交付）

- **P0 合规脱嵌 + 品牌**（✅ 完成 2026-10-22，证据见下）
  1. ✅ `brand.rs`：PRODUCT_NAME/VERSION(0.1.0)/自有类名（LDir.*）/自名配置文件
     （L-Dir.ini / L.qdr）+ 原版文件读取回退（R2 导入特性）；标题、状态栏、
     类名、main.rs、egui 路线全部接品牌常量。
  2. ✅ toolbar.rs 重写：12 个原版 PNG include_bytes → 自绘 48x48 几何 glyph
     （Pen 结构体；layout_prev/next、layout_menu、new_window、view、refresh、
     go、tree_toggle），2 个单元测试。按钮顺序与命令 id 保留（布局体验层）。
  3. ✅ `ui_native/menu.rs`：CreateMenuW 自绘菜单（文件/查看/活动窗口/收藏/
     帮助；自有中文文案、助记符）；id 段 30000+（cmd_id 常量，含 2 个测试：
     段冲突回归 + HMENU 冒烟）。app.rs 不再加载模板 128。
  4. ✅ 关于框：show_about_brand（MessageBoxW，brand 文案）替代原版模板 100；
     res.rs/dialogs.rs 整体删除（含 4 个原版模板测试）。
  5. ✅ commands.rs 重写分发：cmd::（57600=新建文件夹等**推断错**的旧常量）
     删除，按附录 A 行为表实现 FILE_OPEN/SAVE/DESKTOP/DUP + cmd/powershell/
     FileRun/Exit + 视图模式/网格线/列宽/缩放/配色/活动窗格/收藏夹；
     GetOpen/SaveFileNameW 状态对话框（修复过 lpstrDefExt 悬垂指针）；
     窗格按钮段保留并标注 tooltip 直译*待复核*不变。
  6. ✅ shell/system.rs 新增：spawn_terminal / shell_file_run /
     shell_shutdown_windows（IShellDispatch，附录 A.2 确证）。
  验收实测（2026-10-22）：
  - 二进制审计：`ATL:` 计数 0；"Q-Dir" 仅剩 brand.rs/config.rs 的兼容文件名
    与 ini 段名字面量（R2 预期）；PNG 头为 image crate 解码签名表非资源。
  - 端到端冒烟（PostMessage 新 id）：30024→「TEST 视图模式 id=30024 view=1」、
    30025→「网格线切换」、30060→「配色档位」、30080→「活动窗口 #1」、
    30200→弹出 #32770「L-Dir」关于框、39999→「[l-dir] 未接命令」留痕。
  - cargo build 无 error；lib 254 + 集成 11 全绿（删除 7 个 1:1 时代测试、
    新增 4 个自绘测试；基线 257→254-4+5=254 账目吻合）。
- **P1 核心功能对标**（§3 P1 清单，按表逐项，钩子端到端验证）
- **P2 体验完整**（对话框自绘模板 + 行为开关）
- **P3 长尾**（逐项决策）

## 5. 验收定义（替代像素/UIA diff）

每个 P1/P2 功能：①纯函数单测；②进程内 TEST 钩子日志断言（QDIR_TEST_*）；
③行为断言（剪贴板内容/文件落盘/窗口显隐/配置写回读回）。
control_tree 冒烟：主窗层级结构健全（ReBar/状态栏/4 窗格/按钮排存在）。

## 6. 决策项（需要用户拍板，均不阻塞开工）

1. **产品名**：L-Dir（现名候选）/ 其他？影响标题、品牌常量、exe 名。
2. **配置文件名**：沿用 Q-Dir.ini + start.qdr（零迁移，但名字带原版）
   或自名 L-Dir.ini + L.qdr + 一次性导入向导？——我倾向**自名 + 导入**。
3. **图标集风格**：线性/扁平/拟物？P0 先用占位几何图。
4. --egui 对照路线现在删还是留？

> 本文件取代 NATIVE_SPEC.md 的"任务清单"职能；NATIVE_SPEC.md 顶部已标注
> 转为参考档案，其证据（尤其附录行为总表）是 §3 功能清单的来源。
