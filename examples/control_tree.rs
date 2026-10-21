//! 控件树 dump：用 UI Automation 枚举指定 PID 的顶层窗口控件树，输出规范化 JSON。
//!
//! 用途：原版 Q-Dir 与重写版并排跑，各自 dump 一份后 diff，作为"1:1 复刻"的
//! 机器可验收标准（替代逐张截图肉眼比对）。
//!
//! 用法：cargo run --release --example control_tree -- <PID> [输出文件]

use std::collections::VecDeque;

use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::Win32::System::Variant::VT_BSTR;
use windows::Win32::UI::Accessibility::*;
use windows::core::*;

/// 树节点（字段顺序固定 → 输出字节稳定，便于 diff）。
#[derive(serde::Serialize)]
struct Node {
    depth: usize,
    kind: String,
    name: String,
    class: String,
    automation_id: String,
}

/// 取元素的某个 UIA 字符串属性，取不到返回空串。
///
/// UIA 的字符串属性都是 `VT_BSTR`，VARIANT 在 0.62 里是匿名联合体
/// `VARIANT_0`，所以从这里取 `bstrVal`（`ManuallyDrop<BSTR>`）。
fn prop_str(e: &IUIAutomationElement, pid: UIA_PROPERTY_ID) -> String {
    let Ok(v) = (unsafe { e.GetCurrentPropertyValue(pid) }) else {
        return String::new();
    };
    if v.vt() != VT_BSTR {
        return String::new();
    }
    let b: &BSTR = unsafe { &v.Anonymous.Anonymous.Anonymous.bstrVal };
    b.to_string()
}

/// 常见 UIA 控件类型 → 名字。
fn control_type_name(t: UIA_CONTROLTYPE_ID) -> String {
    let s = match t {
        UIA_ButtonControlTypeId => "Button",
        UIA_CheckBoxControlTypeId => "CheckBox",
        UIA_ComboBoxControlTypeId => "ComboBox",
        UIA_EditControlTypeId => "Edit",
        UIA_HyperlinkControlTypeId => "Hyperlink",
        UIA_ImageControlTypeId => "Image",
        UIA_ListItemControlTypeId => "ListItem",
        UIA_ListControlTypeId => "List",
        UIA_MenuControlTypeId => "Menu",
        UIA_MenuBarControlTypeId => "MenuBar",
        UIA_MenuItemControlTypeId => "MenuItem",
        UIA_ProgressBarControlTypeId => "ProgressBar",
        UIA_RadioButtonControlTypeId => "RadioButton",
        UIA_ScrollBarControlTypeId => "ScrollBar",
        UIA_SliderControlTypeId => "Slider",
        UIA_SpinnerControlTypeId => "Spinner",
        UIA_StatusBarControlTypeId => "StatusBar",
        UIA_TabControlTypeId => "Tab",
        UIA_TabItemControlTypeId => "TabItem",
        UIA_TextControlTypeId => "Text",
        UIA_ToolBarControlTypeId => "ToolBar",
        UIA_ToolTipControlTypeId => "ToolTip",
        UIA_TreeControlTypeId => "Tree",
        UIA_TreeItemControlTypeId => "TreeItem",
        UIA_WindowControlTypeId => "Window",
        UIA_PaneControlTypeId => "Pane",
        UIA_GroupControlTypeId => "Group",
        UIA_HeaderControlTypeId => "Header",
        UIA_HeaderItemControlTypeId => "HeaderItem",
        UIA_DataGridControlTypeId => "DataGrid",
        UIA_DataItemControlTypeId => "DataItem",
        UIA_DocumentControlTypeId => "Document",
        UIA_SplitButtonControlTypeId => "SplitButton",
        UIA_CustomControlTypeId => "Custom",
        _ => return format!("Type{}", t.0),
    };
    s.to_string()
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let pid_txt = args.next().expect("用法：control_tree <PID> [输出文件]");
    let pid: u32 = pid_txt
        .parse()
        .unwrap_or_else(|_| panic!("PID 必须是数字：{pid_txt}"));
    let out = args.next();

    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;

        let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)?;
        let root = find_root(&automation, pid)?;
        let true_cond = automation.CreateTrueCondition()?;

        let mut nodes = Vec::new();
        let mut queue: VecDeque<(IUIAutomationElement, usize)> = VecDeque::new();
        queue.push_back((root, 0usize));

        while let Some((el, depth)) = queue.pop_front() {
            if nodes.len() >= 5000 {
                break;
            }
            let ct = el.CurrentControlType().unwrap_or(UIA_CONTROLTYPE_ID(0));
            nodes.push(Node {
                depth,
                kind: control_type_name(ct),
                name: prop_str(&el, UIA_NamePropertyId),
                class: prop_str(&el, UIA_ClassNamePropertyId),
                automation_id: prop_str(&el, UIA_AutomationIdPropertyId),
            });

            // 子节点；不支持枚举的元素直接跳过。
            if let Ok(arr) = el.FindAll(TreeScope_Children, &true_cond) {
                if let Ok(len) = arr.Length() {
                    for i in 0..len {
                        if let Ok(child) = arr.GetElement(i) {
                            queue.push_back((child, depth + 1));
                    }
                }
            }
        }
        }

        let json = serde_json::to_string_pretty(&nodes).expect("序列化控件树失败");
        match out {
            Some(p) => {
                std::fs::write(&p, &json)?;
                println!("[control_tree] 写出 {} 个节点 -> {p}", nodes.len());
            }
            None => println!("{json}"),
        }

        CoUninitialize();
        Ok(())
    }
}

/// 按 PID 找顶层窗口元素。
unsafe fn find_root(automation: &IUIAutomation, pid: u32) -> Result<IUIAutomationElement> {
    let cond = automation.CreateTrueCondition()?;
    let desktop: IUIAutomationElement = automation.GetRootElement()?;
    let arr = unsafe { desktop.FindAll(TreeScope_Children, &cond) }?;
    let len = unsafe { arr.Length() }?;
    for i in 0..len {
        let el = unsafe { arr.GetElement(i) }?;
        if let Ok(v) = unsafe { el.GetCurrentPropertyValue(UIA_ProcessIdPropertyId) } {
            let p = unsafe { v.Anonymous.Anonymous.Anonymous.lVal } as u32;
            if p == pid {
                return Ok(el);
            }
        }
    }
    Err(Error::from(HRESULT(0x80004005u32 as i32)))
}
