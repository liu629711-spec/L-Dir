# 续任开发提示词（Handoff Prompt）— L-Dir / Q-Dir 复刻项目

> 用法：把下方代码块全文粘贴到新会话的第一条消息即可。生成于 2026-10-24，对应基线：345 测试全绿、14 钩子全绿、普通启动健康。

```
你是 Q-Dir 复刻项目（产品名 L-Dir）的续任开发者。项目已完成核心功能补齐与多处深层缺陷修复，现在基于当前状态继续开发。全程用简体中文。

## 一、项目背景与目标
- 目标：复刻 Q-Dir（四格文件管理器）的**全部核心功能**。界面外观可自主设计，但功能、能力、交互行为必须 1:1，核心功能一个都不能丢。
- 技术栈：Rust（edition 2024）+ windows crate 0.62 + 原生 Win32 UI（ui_native 路线）。egui 立即模式路线已废弃，仅存 `--egui` 对照参数，不要走回头路。

## 二、关键路径（先读这些再动手）
- 工程目录：`D:\L-dir\qdir-rs`（cargo 工程；产物 `target/release/qdir-rs.exe`；**当前所有主体改动在工作树未提交，第一步先 git commit 基线**）
- 原版参照（可直接运行观察）：`D:\Q-Dir_Portable_x64\Q-Dir\Q-Dir_x64.exe`（x64，sha256 321d3288cddf37373fb2519711f8ab569cd5a2f9bf9813142de9a05da509a40c）；32 位副本 `D:\Q-Dir\Q-Dir.exe`
- 原版全量反编译伪 C：`D:\qdir_reverse\decompiled\all_functions.c`（6.4MB，grep 引用时记作 `AF:行号`）
- REA 逆向工具：`mcp__rea__open_binary` 绑定上述 exe（需要深挖原版行为时用；输出可能巨大，先 summary 再下钻）
- 规格文档（按序读完再写代码）：
  1. `D:\L-dir\qdir-rs\PRODUCT_SPEC.md` —— 产品总纲、开发规矩 R1–R6、功能清单与进度
  2. `D:\L-dir\qdir-rs\NATIVE_SPEC.md` —— 逆向证据档案；附录 A「命令行为总表」是功能需求的事实来源（含确证/*待复核* 标注）
  3. `D:\L-dir\qdir-rs\FEATURE_GAP_AUDIT.md` —— 75 项功能审计矩阵（原版证据 → 现状 → 缺口 → 严重度），剩余工作从这里的 P1/P2/P3 勾选
  4. `D:\L-dir\qdir-rs\DEV_SPEC_BATCH_B.md` —— 拖放/大图标施工规格（设计决策记录格式参考）

## 三、当前已验证状态（2026-10-24 基线，勿声称回退）
- `cargo test`：**345 全绿**（331 lib + 3 bin + 6 config 集成 + 5 core 集成）
- `cargo build --release`：0 error；普通启动健康（四窗格、ReBar 工具栏、目录树、菜单、状态栏、托盘、设置/规则/布局槽对话框均在）
- 已完成批次：A 键盘与文件操作（Ctrl+C/X/V/A/Del、F2 内联重命名、删除进回收站、新建文件夹唯一命名、Ctrl+T 新标签、下拉选盘导航）；B 拖放三路径（拖出/窗格间/Explorer 拖入）+ 大图标；C 隐藏文件开关、地址栏 SHAutoComplete、命令行目录启动、Alt+←/→ 历史导航、树启动修复；D 文件夹监视、布局槽、列显隐
- 深层缺陷已修：.qdr IDLIST 会话路径恢复（shell/idlist.rs）、SHChangeNotifyEntry packed(1) 步长、shell 通知投递链（改 ReadDirectoryChangesW）、WM_APP 常量撞车、rename_path 完整目标路径、enumerate NUL 拼接、GDI 泄漏

## 四、架构速览
- 依赖方向：core（纯领域类型）← config（ini/qdr UTF-16LE 持久化）← panes/filters/services/shell（Win32 FFI 全封在 shell 层）← ui_native（原生 UI）← main.rs
- `src/ui_native/`：mainframe.rs（主框架，数千行：窗格/布局/命令分发/测试钩子）、subclass.rs（控件子类化：键盘/拖放起点/标签）、commands.rs（WM_COMMAND 分发）、menu.rs（自建菜单，id 段 30000+）、toolbar.rs、splitter.rs（12 种布局模板）、tree.rs、settings_dialog.rs、rules_dialog.rs、slots_dialog.rs、app.rs（WinMain 等价物：CoInit/OleInit/ICC=0x6FF）
- `src/shell/`：enumerate（目录枚举）、fileops（SHFileOperation 复制/移动/删除/重命名，allow_undo 回收站）、clipboard（CF_HDROP+Preferred DropEffect）、dragdrop（begin_drag_ex 双许可、PaneDropTarget 手写 IDropTarget）、icons、contextmenu、tray、special、system、startup、watch（RDC 目录监视）、idlist（.qdr FolderN PIDL 百分号编解码）
- `src/panes/`：PaneController、tree、preview；`src/services/`：export/i18n/favorites；`src/core.rs`+`src/config.rs`：全部配置/会话模型与序列化（含 Filter2 着色规则、SP 万分比分割）
- 品牌与配置文件名：`src/brand.rs`（L-Dir.ini/L.qdr 自名 + 原版兼容回落读取）

## 五、开发规矩（PRODUCT_SPEC R1–R6，违反即返工）
- R1 产品二进制不得内嵌任何原版字节（菜单/对话框模板/PNG/图标/文案）；`assets/extracted`、`assets/pngs`、`assets/menu_texts_zh.txt` 只读参考
- R2 兼容是特性不是枷锁：可导入原版 Q-Dir.ini/start.qdr/Favoriten；产品默认写自己的文件名
- R3 功能范围以 NATIVE_SPEC 附录 A 行为总表为准勾选
- R4 验收 = 纯函数单测 + 进程内 QDIR_TEST_* 钩子实测 + 行为断言（剪贴板内容/磁盘落盘/窗口显隐/配置读写回）
- R5 诚实记录：每个功能标注 已实现+证据 / 部分+缺口 / 待做；推断标 *待复核*
- R6 环境纪律：构建前 `taskkill //IM qdir-rs.exe //F`；原版 Q-Dir 运行会漂移 QDir_Id/Focus/ColsAndSort1 三个 ini 键（测试夹具对这三键只断言合法性）；Git Bash 的 /tmp 对 py 不可见

## 六、验证方法（每项功能必须这样验收，不许只跑 build）
```bash
cd D:/L-dir/qdir-rs
taskkill //IM qdir-rs.exe //F
cargo test                                   # 345 全绿为底线，只增不减
cargo build --release                        # 0 error
# 端到端钩子（启动即自跑自断言，panic=失败；exit 124=超时杀进程=存活正常，139=段错误）：
env QDIR_TEST_KEYS=1         timeout 20 ./target/release/qdir-rs.exe   # Ctrl+C 剪贴板 + Ctrl+A 全选
env QDIR_TEST_CLIP=1         timeout 20 ./target/release/qdir-rs.exe   # Ctrl+X/V 移动落盘
env QDIR_TEST_OPS=1          timeout 20 ./target/release/qdir-rs.exe   # 新建文件夹唯一化/新标签/Del 回收站
env QDIR_TEST_RENAME=1       timeout 20 ./target/release/qdir-rs.exe   # F2 内联改名磁盘断言
env QDIR_TEST_HISTORY=1      timeout 20 ./target/release/qdir-rs.exe   # 后退/前进 history_pos+地址栏
env QDIR_TEST_VIEW_LARGE=1   timeout 20 ./target/release/qdir-rs.exe   # 大图标 LVSIL_NORMAL
env QDIR_TEST_DND=1          timeout 20 ./target/release/qdir-rs.exe   # 落点队列注入（移动）
env QDIR_TEST_DROPFILES=1    timeout 20 ./target/release/qdir-rs.exe   # WM_DROPFILES 兜底（复制）
env QDIR_TEST_AUTOCOMPLETE=1 timeout 20 ./target/release/qdir-rs.exe   # 地址栏补全挂载
env QDIR_TEST_COMBO=1        timeout 20 ./target/release/qdir-rs.exe   # 选盘导航
env QDIR_TEST_TAB_CLOSE=1    timeout 20 ./target/release/qdir-rs.exe   # 中键关标签
env QDIR_TEST_WATCH=1        timeout 20 ./target/release/qdir-rs.exe   # 目录监视增删自动刷新
env QDIR_TEST_SLOTS=1        timeout 20 ./target/release/qdir-rs.exe   # 布局槽存/载/删+路径恢复
env QDIR_TEST_COLS=1         timeout 20 ./target/release/qdir-rs.exe   # 列显隐 5→4→5
```

## 七、血泪教训（这些坑都实际踩过，直接规避）
1. **windows crate 0.62 的 `SHChangeNotifyEntry` 是 `#[repr(C, packed(1))]`（12 字节步长）**——自定义同布局结构体必须同样 packed(1)；用 repr(C)（16 字节步长）时条目 ≥2 shell 从偏移 12 读到全零 padding 当指针 → SIGSEGV（曾致普通启动即崩）
2. **自定义消息常量防撞车**：`WM_TRAY=0x8001`（历史硬编码）、`WM_APP_DROP=0x8002` 已占用；新消息从 `WM_APP+3` 起。撞车症状=消息到达窗口过程但走错分支、静默失效
3. **本机 `SHChangeNotifyRegister` 注册成功但零投递**（窗口类型×消息段×事件掩码×递归全矩阵实测无效，根因未明）——目录监视已改用 `ReadDirectoryChangesW`（shell/watch.rs，每路径一个 overlapped 后台线程），勿回退
4. **未经运行验证的"完成汇报"不可信**——本项目曾带着"普通启动即段错误"的代码汇报 IS_PASS；你必须亲自跑钩子验收
5. `SHFileOperationW` 的 pTo 必须传完整目标路径（只传裸文件名返回 DE_DIFFDIR）
6. `FindFirstFileW` 模式串：先去掉尾部 NUL 再拼 `*`（NUL 之后拼的字符会被 C 字符串截断）
7. 跨进程 `SendMessageW` 传本地结构指针会崩目标进程——验证一律走进程内 QDIR_TEST 钩子（同进程传指针安全）
8. 系统图标索引在大小图像列表间通用；按索引取大图标无可靠公开 API，走 SHGetFileInfoW 的系统图像列表
9. `RegisterDragDrop`/`SHChangeNotifyRegister` 都不能在 WM_CREATE 内调用（ole32 0xC000041D / 段错误）——用 `SetTimer(0ms)` 延到消息循环后；同理，on_create 内的测试钩子若需要泵消息等待，必须先创建定时器再等待
10. 手写 COM vtable 对象必须 `#[repr(C)]` 且 Box 稳定地址后再注册；销毁先 revoke 再释放
11. `IContextMenu` 的 owner-draw 扩展需要 WH_MSGFILTER hook（未实现，当前降级为仅注入普通菜单项）

## 八、剩余工作（从 FEATURE_GAP_AUDIT.md 勾选，按优先级）

### P1 核心体验收尾
1. **Ctrl+Z 撤销删除**：回收站还原（原版「撤消 删除 Ctrl+Z」menu_texts:79 确证；删除已默认进回收站，缺的是还原）。无简单公开 API，先出设计（删除记录+回收站定位还原）再动手
2. **拖出默认语义实测复核**：当前 Ctrl→COPY、默认/Shift→MOVE（Q-Dir 惯例推断 *待复核*）——与原版并开用真实鼠标比对
3. **Explorer 真实拖入**：IDropTarget COM 回调路径未经真实鼠标触发（现有覆盖=QI/refcount 单测+进程内队列注入）
4. **布局槽原版 UI 入口**：当前为自绘对话框（保存/载入/删除/复制路径 + `<exe>/L-Slots/slot-N.qdr` 存储）；原版入口未逐条对照（附录 A.7 机制已确证：单击载入/Ctrl 复制路径/Shift 另存/Del 删除）

### P2 体验完整
5. **分组视图 GroupBy**：core 的 group_by/columns_plus_view 已解析零消费；原版视图状态串 `GroupBy,Ext;Group,1;` 已建模
6. **列配置持久化对齐原版**：现用自名 ini 键；原版是 772 字节 `Column_OS_10.0_PloderN` blob（filters/mod.rs 只解出 GUID 锚定列条目表；IconSize/FFlags/排序方向因 blob 内 1 字节错位未取证，属 H2(a) 遗留）
7. **每窗格独立视图模式**：现 set_view_mode_all 全窗格一刀切（附录 A.5 称原版作用于活动窗格）
8. **打印**：PrintDlgW/OpenPrinterW/GetPrinterW 全未接（现 print_file 仅 ShellExecuteW("print")）
9. **窗口排列**：层叠/并排/上下/排列图标（i18n 文案已在，实现未接）
10. **地址栏历史下拉**：ComboBox 现仅驱动器列表，无导航历史下拉（go.png 0xB379 下拉语义）

### P3 长尾
11. 多语言 UI（services/i18n.rs 三语表已在；原版是 IMG 资源算术编码语言块，FUN_004793f4 range coder 未逆）
12. 注册表扩展 `\shell\*Q-Dir`（安装期行为；Win32_System_Registry 特性已启用）
13. Q-Dir_Tmp\、color.ini、启动耗时状态栏、TrayNotifyWnd、UnhandledExceptionFilter
14. 单实例互斥体——**注意**：原版有 CreateMutexW"Q-DIR-GUID"（AF:168214）但同时支持「重复窗口=启动新实例」（0xE100 确证），用途存疑；做前先复核，防止自相矛盾

## 九、工作方式
- 每批改动：读规矩 → 纯函数+单测 → 接 UI → 加/复用 QDIR_TEST 钩子实测 → cargo test 只增不减 → 汇报带证据日志行（`[qdir-rs] TEST ...`）
- 大功能先落盘设计文档（照 DEV_SPEC_BATCH_B.md 格式），防会话中断丢失
- 规格含糊：按 Q-Dir 同类惯例就近实现并标注 *待复核*，不要卡住
- 原版行为存疑：直接运行原版 exe 观察 / grep all_functions.c / 开 REA 会话，三选一
- 不要 git 提交除非用户要求；不要动 PRODUCT_SPEC/NATIVE_SPEC 的既有结论（增量追加可以）
```
