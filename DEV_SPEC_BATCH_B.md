# 批次B 施工规格：拖放三路径 + 大图标视图（team-lead 合成，2026-10-23）

> 依据 FEATURE_GAP_AUDIT.md #27/#28/#29/#34 与对现有封装的逐行核对。
> 施工方：工程师。前置：批次A 已合入且 cargo test 全绿。

## B1 大图标视图出图（#34，最简，先做）

现状（已核对）：
- `mainframe.rs:3486 system_small_imagelist()`：`SHGetFileInfoW(SHGFI_SYSICONINDEX|SHGFI_SMALLICON)` 取系统小图列表，`mainframe.rs:3008` 挂 `LVM_SETIMAGELIST(LVSIL_SMALL)`。
- `FileItem.icon_index` = 系统图标索引（enumerate.rs:286 `SHGFI_SMALLICON|SHGFI_SYSICONINDEX|SHGFI_USEFILEATTRIBUTES`）——**系统图标索引在大小两个图像列表间通用**（同一 iIcon 对大小表都有效）。
- `set_view_mode_all(mode)`（mainframe.rs:1178）已发 LVM_SETVIEW。
- `core::ViewMode` 有 LargeIcons 档；ImageSize/image_size 键已建模（core.rs:903）。

实施：
1. 新增 `unsafe fn system_large_imagelist() -> Option<HIMAGELIST>`（mainframe.rs:3486 同款，flag 换 `SHGFI_LARGEICON`），句柄缓存一次（OnceLock 或 frame 字段）。
2. `create_pane`（mainframe.rs:2996 起的 lv 初始化块）加 `LVM_SETIMAGELIST(LVSIL_NORMAL, large_himl)`。
3. 切到大图标视图时确认 LVS_ICON 生效（`set_view_mode_all` 已有；LVSIL_NORMAL 存在时 comctl32 自动用大图渲染）。
4. 验证：`QDIR_TEST_VIEW_LARGE` 钩子：LVM_SETVIEW(LV_VIEW_ICON) → LVM_GETVIEW 回读 + LVM_GETIMAGELIST(LVSIL_NORMAL) 非空 + 启动后 UIA/EnumChildWindows 截图级确认（或至少 GetView 断言）。

## B2 begin_drag 双许可 + 按键解析 + CF_HDROP（#27 拖出）

现状（已核对）：
- `dragdrop.rs:170 begin_drag(hwnd, paths, cut)`：`allowed = if cut {MOVE} else {COPY}` 单值；`set_preferred_drop_effect(&data, effect)` 单值；SimpleDropSource 手写 vtable；`do_drag_drop_raw` 原始 ABI 绑定；`ensure_com_apartment` 已处理 STA。
- `clipboard.rs` 有 CF_HDROP 读写封装（写侧可参考其 DROPFILES 结构构造）。

实施：
1. `begin_drag` 保持原签名不动（兼容），新增 `begin_drag_ex(hwnd, paths, allowed: u32, preferred: u32)`：`dwokeffects=allowed`（COPY|MOVE 双许可），Preferred DropEffect 按 preferred 写；`begin_drag` 改为薄包装（cut→(MOVE,MOVE)/else→(COPY,COPY)）。
2. `begin_drag_ex` 内 `set_preferred_drop_effect` 之后，再 `IDataObject::SetData` 一份 **CF_HDROP**（DROPFILES：头 20 字节 + 双 NUL 结尾宽串数组，句柄 GlobalAlloc(GMEM_MOVEABLE|GMEM_ZEROINIT)）——让本进程 IDropTarget 与任何 CF_HDROP 阅读器都能取到路径（原 CIDLData 只有 CFSTR_SHELLIDLIST）。
3. 按键解析（拖出起点）：subclass list WM_LBUTTONDOWN 记 `drag_start: POINT + paths`；WM_MOUSEMOVE 左键且位移 > GetSystemMetrics(SM_CXDRAG/SM_CYDRAG) → 调 `begin_drag_ex`：Ctrl 按下→(COPY|MOVE, COPY)；Shift→(COPY|MOVE, MOVE)；默认→(COPY|MOVE, MOVE)（Q-Dir 窗格间默认移动；拖到外部由目标按 preferred 决议——**待用户实测复核**，注释标注）。
4. 拖放结束后按 DragOutcome 刷新源窗格（Move 成功→重灌；Copy→不动）。
5. 阻塞语义：DoDragDrop 在子类 WM_MOUSEMOVE 内运行模态循环属正常用法；确保 ReleaseCapture 先调用。

## B3 落点：PaneDropTarget（IDropTarget 手写 COM）+ 注册/撤消（#28/#29）

现状（已核对）：
- `dragdrop.rs:467 SimpleDropSource` 手写 COM 模式可照抄；`register_drop_target(hwnd, target_raw)`（:395）与 `revoke_drop_target(hwnd)`（:418）已封装 ole32 RegisterDragDrop/RevokeDragDrop。
- `fileops.rs`：`copy_files(hwnd, sources, dest_dir, opts)` / `move_files(...)` / `move_files_with_config` 齐备；`FileOpOptions` 可设 allow_undo。
- `mainframe.rs` 有 per-pane `list` 句柄数组与 fill_pane。

实施：
1. `dragdrop.rs` 新增 `PaneDropTarget`（手写 vtable，严格照 SimpleDropSource 的 refcount/QueryInterface 模式）：
   - vtable 槽位：[0..2] IUnknown，[3] DragEnter(pDataObj,grfKeyState,pt,pdwEffect), [4] DragOver, [5] DragLeave, [6] Drop。
   - 构造参数：pane_index + 一个 `Rc<RefCell<PaneDropCtx>>` 或裸指针回指 MainFrame（**回指方案需与主框架线程模型一致**；若拿不到安全回指，最小可用版：DragEnter/DragOver 只写 *pdwEffect（按 Preferred DropEffect 或 Ctrl/Shift 决议 COPY/MOVE，无条件返回 S_OK），Drop 时把 (paths, effect, screen_pt) 通过一个全局/线程局部队列抛给主线程，由主框架消息循环在 Drop 返回后消费——**推荐此方案，避免 COM 回调里直接操作 UI**）。
   - Drop 路径解析：从 IDataObject 读 CF_HDROP（wm clipboard.rs 的读取封装，注意 DataObject 侧的 GetData 而非 OleGetClipboard）→ paths；effect：DROPEFFECT_MOVE→move_files，COPY→copy_files；dest_dir = 该窗格当前 tab.path；冲突走 opts（确认对话框已有 FileOpsConfig.move_file_confirmation）。
   - DragLeave：清 hover 高亮（可先只清队列）。
2. MainFrame 每窗格 list 建窗后 `register_drop_target(list_hwnd, target.as_interface_ptr())`；target 对象存入 frame（保活）；WM_DESTROY/on_destroy 路径 `revoke_drop_target`。注册失败（DRAGDROP_E_ALREADYREGISTERED 等）打印留痕即可，不影响主流程。
3. 主线程消费队列：主框架加一个自定义消息（WM_APP+2）或在 Drop 后由 PostMessage 触发 `on_dropped_paths(pane, paths, effect)` → fileops + fill_pane。
4. 兜底路径：`accept_drop_files(list_hwnd, true)` + `WM_DROPFILES` 处理（DropFiles::from_raw→paths→同样 on_dropped_paths）。两条路径互斥兜底（OLE 注册成功时 Explorer 不会发 WM_DROPFILES；注册失败时 WM_DROPFILES 仍工作）。
5. 同进程自拖：B2 注入的 CF_HDROP 使 pane→pane 与 pane→外部统一可读。
6. 验证钩子：`QDIR_TEST_DND`：启动自动选中窗格0 第 0 行 → 进程内对窗格1 的 IDropTarget 直接调 Drop（构造内存 IDataObject 走 clipboard.rs 写入路径，或经自定义消息注入 paths 列表）→ 断言文件在窗格1 目录落盘 + 窗格重灌。**拖出（DoDragDrop 模态）与真实鼠标轨迹无法完全自动化，标注 *待实测*，用单元测试覆盖 Drop 后的决策纯函数（effect 决议、同目录拒绝、源==目标拒绝）。**

## 通用验收
- 纯函数单测：effect 决议（按键→preferred）、拖放落点目标窗格判定（screen_pt→pane index）、同目录/自拖拒绝。
- `cargo test` 全绿；`cargo build --release` 无 error。
- 每项钩子实测日志抄录；无法自动化的明确标注。
- 完成后汇报：B1/B2/B3 状态+证据+IS_PASS。
