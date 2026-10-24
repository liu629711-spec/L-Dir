# P1.1 施工规格：Ctrl+Z 撤销删除（回收站定位还原）（2026-10-24）

> 依据 FEATURE_GAP_AUDIT.md #18/#19 与原版菜单文案 `assets/menu_texts_zh.txt:79`
> 「撤消 删除(U)\tCtrl+Z」（视图菜单内，紧跟「粘贴(P)」之后——撤销对象是
> **删除**，与文件操作对话框语义一致；AF 未见 undo 串互证，*待复核*）。
> 前置：批次 A–D 基线已提交（bcad56e），345 测试全绿。

## 0. 问题定义

- 现状：Del 键 / 命令按钮 bi=4 已走 `delete_files(allow_undo=true)` 进回收站
  （mainframe.rs `KbAction::Delete`、commands.rs:520），但**没有任何还原路径**；
  Ctrl+Z 未绑定；删除批次无记录。
- 原版行为（确证面）：菜单项「撤消 删除 Ctrl+Z」存在（menu_texts:79）。
  撤销粒度、是否多级、冲突处理均未反编译确证 —— 按 Explorer 同类惯例实现，
  全部标注 *待复核*。

## 1. 方案选型

**选定：本地删除批次记录 + 回收站 `$I`/`$R` 定位还原。**

| 备选 | 否决理由 |
|---|---|
| shell 回收站右键「还原」动词（IContextMenu） | 还原项定位需要逐项枚举 PIDL + 显示名匹配，且「还原」无公开稳定 verb 字符串，位置序随系统语言变 —— 不可测试 |
| IFileOperation 撤销栈 | 无公开 API 暴露已执行操作的 undo 记录 |
| SHFileOperationW FOF_ALLOWUNDO「自动兜底」 | 只在删除时生效；还原必须自己定位 |

`$I`/`$R` 格式为取证界公开稳定格式（本机 Win10/11 为 v2）：

```
$I 文件（元数据，位于 <盘>:\$Recycle.Bin\<SID>\）：
  v1（Vista–8.1）：[0..8) u64 版本=1 | [8..16) u64 原大小 | [16..24) u64 删除时刻
                   （FILETIME，UTC，100ns） | [24..544) UTF-16LE 原完整路径（260 wchar，NUL 补齐）
  v2（Win10+）  ：[0..8) u64 版本=2 | [8..16) u64 | [16..24) u64 | [24..28) u32 名长 |
                   [28..)   UTF-16LE 原完整路径（名长字节含 NUL；解析按「读剩余全部，
                   首个 NUL 截断」防御两种口径）
$R 文件（数据）：与 $I 同名，I→R（如 $I89YT3C.txt ↔ $R89YT3C.txt）
```

还原 = `MoveFileW($R路径, 原路径)`（同盘即改名，元数据/时间戳自动保留；
$Recycle.Bin 对本用户 SID 目录可读，其他 SID 目录读失败即跳过）。
枚举用 `std::fs::read_dir`（隐藏/系统属性不拦 std API），FFI 面只留 MoveFileW。

## 2. 数据模型与纯函数（shell/recycle.rs，新增）

- `DeleteBatch { t0_filetime: u64, paths: Vec<PathBuf>, pane: usize }` ——
  一次删除操作（键盘 Del 或按钮）= 一个批次。t0 在调 delete_files **之前**
  采 `GetSystemTimeAsFileTime`。
- `system_time_to_filetime(SystemTime) -> u64`（纯，单测：UNIX_EPOCH →
  11644473600_0000000）。
- `parse_i_file(bytes) -> Option<RecycleEntry>`（纯，单测 v1/v2/截断/坏版本）：
  `RecycleEntry { i_path, r_path, original: String, size: u64, deleted_filetime: u64 }`。
- `drive_anchor_of(path) -> Option<PathBuf>`（纯：`C:\a` → `C:\`；相对路径 → None）。
- `pick_restore_entry(path, t0, candidates) -> Option<&RecycleEntry>`（纯）：
  原路径**大小写不敏感**相等 且 `deleted_filetime >= t0 - SLACK(1s)` 的候选取
  `deleted_filetime` 最大者（同路径多次删除只还原最新）。
- `unique_restore_name(target) -> PathBuf`（fs 存在性查询：占用则
  `名 (n).扩展名` 递增；无扩展名回落 `名 (n)`）。
- `scan_drive_bin(anchor, t0) -> Vec<RecycleEntry>`：枚举
  `<anchor>\$Recycle.Bin\<*\>\$I*`（SID 子目录逐个试读，失败跳过；只收
  `deleted_filetime >= t0 - SLACK` 的条目控制扫描量）。
- `restore_batch(&DeleteBatch) -> RestoreReport { restored, failed: Vec<(路径,原因)> }`：
  每路径按锚盘扫描（批内同盘缓存）→ pick → 目标存在则 `unique_restore_name`
  → `MoveFileW`。找不到/锚盘缺/移动失败进 failed，不中断其余路径。

## 3. UI 接线（ui_native）

1. `MainFrame.undo_deletes: Vec<DeleteBatch>`（上限 32，超出丢最旧）。
2. **删除统一入口** `delete_selection_with_undo(pidx, main) -> bool`：
   t0 → `delete_files(allow_undo=true)` → 成功 push 批次 + `fill_pane`。
   `KbAction::Delete` 与 commands.rs bi=4（按钮删除）都改走此入口（此前两处
   重复代码收敛；语义不变）。
3. `KbAction::UndoDelete` 新档（subclass `ctrl_list_action` 加 VK_Z）→
   `undo_last_delete(main)`：pop 栈顶 → `restore_batch` → 恢复数>0 时
   `fill_pane(batch.pane)`（若窗格已导航走，RDC 监视会刷新显示中的还原目录）
   + 留痕日志；空栈返回 false 留痕。
4. 菜单：查看菜单加「撤消 删除(&U)\tCtrl+Z」`VIEW_UNDO_DELETE=30047`
   （原版位置=视图菜单粘贴之后；文案按原版语义自拟，R1）。
5. **顺带修复 id 撞车**（本次核查发现）：`menu.rs` 里 `COL_BASE=30040` 与
   `ZOOM_BASE=30040` 同值——列子菜单 30040–30044 的 dispatch 臂排在缩放臂
   之前（commands.rs:285 vs :451），**缩放 60/80/100/110/120% 五档会被列
   显隐吞掉**（Rust match 先中先得）。修法：`COL_BASE` → **30110**
   （30110–30114，避让 30090–30101 布局/收藏段与 30040–30050 缩放段），
   menu.rs id 冲突测试补「全部命令段两两不交」断言。

## 4. 验收（R4）

- 单测：parse_i_file v1/v2/截断/坏版本、pick_restore（旧时间排除/大小写/
  最大时刻）、drive_anchor_of、unique_restore_name（占用递增/无扩展名）、
  system_time_to_filetime 已知值。
- 钩子 `QDIR_TEST_UNDO=1`（undo_probe）：临时目录建 `撤销甲.txt`(AAA) +
  `撤销子目录\撤销乙.txt`(BBB) → 窗格1 导航 → 多选两行 → 全链 Del
  （synth_plain_key）断言双消失 → 全链 Ctrl+Z（synth_ctrl_key VK_Z）断言
  双还原 + 内容一致（目录内文件一并回来）→ 再 Ctrl+Z 空栈拒绝留痕 →
  清理临时目录。
- `cargo test` 全绿（345 → 只增不减）；`cargo build --release` 0 error；
  钩子日志逐行抄录。
- 无法自动化项：多级撤销连续恢复的历史批次（栈序）由单测覆盖批次结构，
  连续真实操作 *待实测*；跨盘删除的还原（bin 各盘独立扫描已实现）*待实测*。

## 5. 风险与边界

- 删除时若目标盘无回收站（如禁用/闪存盘）→ 文件直接消失，还原报
  「回收站未找到」不崩溃（与系统行为一致）。
- 同名冲突（原路径已被重建）：还原到 `名 (n).扩展名`，不覆盖 ——
  Explorer 同场景弹对话框询问，我们简化为自动改名 *待复核*。
- 撤销不回收「撤销」本身（再按 Del 可再删；不做 redo，原版菜单亦无）。
