//! 罗技虚拟HID设备管理库
//! 提供创建、销毁虚拟设备及发送输入事件的功能

use std::ffi::CString;
use std::ptr;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use anyhow::Result;

use windows::Win32::Foundation::{CloseHandle, HANDLE, POINT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
};

mod constants;
mod device_discovery;
mod driver_manager;
mod hid_manager;
mod types;
mod utils;

use crate::device_discovery::DeviceDiscovery;
use crate::types::{DeviceIds, KeyboardInput, MouseInput};

static DEVICE_MANAGER: OnceLock<Mutex<DeviceHandleManager>> = OnceLock::new();

/// 获取或初始化全局的设备管理器
fn get_manager() -> &'static Mutex<DeviceHandleManager> {
    DEVICE_MANAGER.get_or_init(|| Mutex::new(DeviceHandleManager::new().unwrap()))
}

/// 设备句柄管理器
struct DeviceHandleManager {
    bus_handle: HANDLE,
    device_handle: HANDLE,
    keyboard_id: Option<u32>,
    mouse_id: Option<u32>,

    // --- 状态 ---
    mouse_button_state: u8,
    keyboard_modifier_state: u8,
    keyboard_keys_state: [u8; 6],
}

impl DeviceHandleManager {
    fn new() -> Result<Self> {
        let bus_handle = hid_manager::open_bus_device()?;
        let device_handle = hid_manager::open_vulnerable_device()?;

        Ok(Self {
            bus_handle,
            device_handle,
            keyboard_id: None,
            mouse_id: None,
            mouse_button_state: 0,
            keyboard_modifier_state: 0,
            keyboard_keys_state: [0; 6],
        })
    }

    /// 确保设备已创建或被发现，并将ID存储在实例中
    fn ensure_devices_created(&mut self) -> Result<()> {
        if self.keyboard_id.is_some() || self.mouse_id.is_some() {
            return Ok(());
        }

        // 首先，尝试发现已存在的设备
        let mut device_ids = DeviceDiscovery::discover_devices()?;
        if device_ids.is_empty() {
            // 如果没发现，则创建它们
            println!("[*] 未发现现有设备，开始创建流程...");
            device_ids = hid_manager::create_hid_devices()?;
        } else {
            println!("[*] 已发现现有虚拟设备。");
        }

        self.keyboard_id = device_ids.keyboard_id;
        self.mouse_id = device_ids.mouse_id;

        Ok(())
    }
}

impl Drop for DeviceHandleManager {
    fn drop(&mut self) {
        // let device_ids = DeviceIds {
        //     keyboard_id: self.keyboard_id,
        //     mouse_id: self.mouse_id,
        // };
        // if !device_ids.is_empty() {
        //     // 调用 destroy_hid_devices 来执行销毁逻辑
        //     if let Err(e) = hid_manager::destroy_hid_devices(&device_ids) {
        //         println!("[!] 销毁设备时出错: {}", e);
        //     }
        // }

        // 关闭句柄
        if !self.device_handle.is_invalid() {
            unsafe { CloseHandle(self.device_handle).ok() };
        }
        if !self.bus_handle.is_invalid() {
            unsafe { CloseHandle(self.bus_handle).ok() };
        }
    }
}
unsafe impl Send for DeviceHandleManager {}

/// 结果状态码
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub enum VHidResult {
    Success = 0,
    Error = 1,
    DeviceNotFound = 2,
    AccessDenied = 3,
    InvalidParameter = 4,
    NotInitialized = 5,
}

/// 初始化虚拟设备系统
/// 返回: 状态码
#[no_mangle]
pub extern "C" fn vhid_initialize() -> VHidResult {
    // 第一次调用 get_manager() 会执行初始化
    let _ = get_manager();
    VHidResult::Success
}

/// 清理虚拟设备系统
/// 注意：由于 OnceLock 无法被安全地“卸载”，这个函数实际上不会销毁
/// DeviceHandleManager。操作系统会在进程结束时自动清理句柄。
/// 这个函数为了API兼容性而保留。
#[no_mangle]
pub extern "C" fn vhid_cleanup() -> VHidResult {
    // 理论上我们可以在这里销毁设备，但 manager 实例会一直存在直到程序结束。
    // Drop 会在程序结束时自动处理清理工作。
    VHidResult::Success
}

/// 激活虚拟设备。
/// 这个函数会确保虚拟键盘和鼠标被创建或被发现，并准备好接收输入。
/// 这是发送任何输入之前的必要步骤。
#[no_mangle]
pub extern "C" fn vhid_power_on() -> VHidResult {
    let mut manager = get_manager().lock().unwrap();
    match manager.ensure_devices_created() {
        Ok(_) => VHidResult::Success,
        Err(_) => VHidResult::Error,
    }
}

/// 停用虚拟设备。
/// 这个函数会从系统中移除虚拟键盘和鼠标，但驱动本身保持加载状态。
/// 调用此函数后，将无法再发送输入，直到下一次调用 vhid_power_on。
#[no_mangle]
pub extern "C" fn vhid_power_off() -> VHidResult {
    let mut manager = get_manager().lock().unwrap();
    let device_ids = DeviceIds {
        keyboard_id: manager.keyboard_id,
        mouse_id: manager.mouse_id,
    };

    if device_ids.is_empty() {
        // 如果设备本就不存在，也视为成功
        return VHidResult::Success;
    }

    if hid_manager::destroy_hid_devices(&device_ids).is_ok() {
        // 成功销毁后，清空内部状态
        manager.keyboard_id = None;
        manager.mouse_id = None;
        VHidResult::Success
    } else {
        VHidResult::Error
    }
}

/// 发送一个完整的鼠标报告
#[no_mangle]
pub extern "C" fn vhid_send_mouse_report(report: *const MouseInput) -> VHidResult {
    if report.is_null() {
        return VHidResult::InvalidParameter;
    }
    let manager = get_manager().lock().unwrap();
    if manager.device_handle.is_invalid() {
        return VHidResult::NotInitialized;
    }
    // 直接调用 hid_manager 的底层函数
    match unsafe { hid_manager::send_mouse_input(manager.device_handle, &*report) } {
        Ok(_) => VHidResult::Success,
        Err(_) => VHidResult::Error,
    }
}

/// 发送一个完整的键盘报告
#[no_mangle]
pub extern "C" fn vhid_send_keyboard_report(report: *const KeyboardInput) -> VHidResult {
    if report.is_null() {
        return VHidResult::InvalidParameter;
    }
    let manager = get_manager().lock().unwrap();
    if manager.device_handle.is_invalid() {
        return VHidResult::NotInitialized;
    }
    match unsafe { hid_manager::send_keyboard_input(manager.device_handle, &*report) } {
        Ok(_) => VHidResult::Success,
        Err(_) => VHidResult::Error,
    }
}

/// 获取最后错误信息
#[no_mangle]
pub extern "C" fn vhid_get_last_error(buffer: *mut i8, size: usize) -> usize {
    if buffer.is_null() || size == 0 {
        return 0;
    }
    let error_msg = utils::get_last_error();
    let c_string = CString::new(error_msg).unwrap_or_default();
    let bytes = c_string.as_bytes_with_nul();
    let copy_size = std::cmp::min(size, bytes.len());
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), buffer as *mut u8, copy_size);
    }
    copy_size.saturating_sub(1)
}

/// 将鼠标移动到屏幕上的绝对坐标位置。
#[no_mangle]
pub extern "C" fn vhid_mouse_move_absolute(x: i32, y: i32) -> VHidResult {
    // 获取屏幕尺寸以进行边界检查
    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };

    let target_x = x.clamp(0, screen_width - 1);
    let target_y = y.clamp(0, screen_height - 1);

    // 设置一个超时，以防万一卡在循环中
    let timeout = Instant::now() + std::time::Duration::from_secs(3);

    loop {
        // 1. 获取当前鼠标的真实位置
        let mut current_pos = POINT { x: 0, y: 0 };
        if unsafe { GetCursorPos(&mut current_pos) }.is_err() {
            return VHidResult::Error;
        }

        // 2. 计算到目标的剩余向量
        let dx = target_x - current_pos.x;
        let dy = target_y - current_pos.y;

        // 3. 如果已经非常接近或到达目标，则成功退出
        // 使用一个小的容差范围 (e.g., 1像素) 来避免在目标点附近微小振动
        if dx.abs() <= 1 && dy.abs() <= 1 {
            break;
        }

        // 4. 计算本次要移动的步长
        // 这是最关键的简化：直接朝着目标移动，步长最大为127
        let move_x = dx.clamp(-127, 127) as i8;
        let move_y = dy.clamp(-127, 127) as i8;

        // 5. 发送移动指令，并保持当前的按键状态
        if vhid_mouse_move(move_x, move_y) != VHidResult::Success {
            return VHidResult::Error;
        }

        // 6. 检查是否超时
        if Instant::now() > timeout {
            return VHidResult::Error; // 移动超时，可能被卡住
        }

        // 7. 给操作系统和目标应用程序足够的反应时间
        // 8ms 对应 125Hz 的刷新率，是一个比较安全和流畅的间隔
        std::thread::sleep(std::time::Duration::from_millis(8));
    }

    VHidResult::Success
}
/// 移动鼠标，同时保持当前的按键状态
#[no_mangle]
pub extern "C" fn vhid_mouse_move(x: i8, y: i8) -> VHidResult {
    let manager = get_manager().lock().unwrap();
    let report = MouseInput {
        button: manager.mouse_button_state as i8,
        x,
        y,
        wheel: 0,
        reserved: 0,
    };
    if hid_manager::send_mouse_input(manager.device_handle, &report).is_ok() {
        VHidResult::Success
    } else {
        VHidResult::Error
    }
}

/// 按下指定的鼠标按键
#[no_mangle]
pub extern "C" fn vhid_mouse_down(button: i8) -> VHidResult {
    let mut manager = get_manager().lock().unwrap();
    manager.mouse_button_state |= button as u8; // 更新状态
    let report = MouseInput {
        button: manager.mouse_button_state as i8,
        x: 0,
        y: 0,
        wheel: 0,
        reserved: 0,
    };
    if hid_manager::send_mouse_input(manager.device_handle, &report).is_ok() {
        VHidResult::Success
    } else {
        VHidResult::Error
    }
}

/// 释放指定的鼠标按键。
#[no_mangle]
pub extern "C" fn vhid_mouse_up(button: i8) -> VHidResult {
    let mut manager = get_manager().lock().unwrap();
    manager.mouse_button_state &= !(button as u8); // 更新状态
    let report = MouseInput {
        button: manager.mouse_button_state as i8,
        x: 0,
        y: 0,
        wheel: 0,
        reserved: 0,
    };
    if hid_manager::send_mouse_input(manager.device_handle, &report).is_ok() {
        VHidResult::Success
    } else {
        VHidResult::Error
    }
}

/// 点按（按下并立即释放）一个鼠标按键。
#[no_mangle]
pub extern "C" fn vhid_mouse_click(button: i8) -> VHidResult {
    if vhid_mouse_down(button) != VHidResult::Success {
        return VHidResult::Error;
    }
    std::thread::sleep(std::time::Duration::from_millis(20));
    vhid_mouse_up(button)
}

/// 滚动鼠标滚轮，同时保持当前的按键状态。
#[no_mangle]
pub extern "C" fn vhid_mouse_wheel(wheel: i8) -> VHidResult {
    let manager = get_manager().lock().unwrap();
    let report = MouseInput {
        button: manager.mouse_button_state as i8,
        x: 0,
        y: 0,
        wheel,
        reserved: 0,
    };
    if hid_manager::send_mouse_input(manager.device_handle, &report).is_ok() {
        VHidResult::Success
    } else {
        VHidResult::Error
    }
}

/// 按下指定的键盘按键（非修饰键）。
/// 最多支持同时按下6个常规按键。
#[no_mangle]
pub extern "C" fn vhid_key_down(key: u8) -> VHidResult {
    let mut manager = get_manager().lock().unwrap();
    // 找到一个空位来存放新的按键
    if let Some(slot) = manager.keyboard_keys_state.iter_mut().find(|k| **k == 0) {
        *slot = key;
    } else {
        // 如果没有空位，可以根据需要决定是忽略还是返回错误
        // 这里我们选择忽略，以避免数组溢出
    }

    let report = KeyboardInput {
        modifiers: manager.keyboard_modifier_state,
        reserved: 0,
        keys: manager.keyboard_keys_state,
    };

    if hid_manager::send_keyboard_input(manager.device_handle, &report).is_ok() {
        VHidResult::Success
    } else {
        VHidResult::Error
    }
}

/// 释放指定的键盘按键（非修饰键）。
#[no_mangle]
pub extern "C" fn vhid_key_up(key: u8) -> VHidResult {
    let mut manager = get_manager().lock().unwrap();
    // 从状态数组中移除指定的按键
    if let Some(slot) = manager.keyboard_keys_state.iter_mut().find(|k| **k == key) {
        *slot = 0;
    }
    // 重新整理数组，将所有非零值前移（可选，但更规范）
    manager
        .keyboard_keys_state
        .sort_unstable_by(|a, b| b.cmp(a));

    let report = KeyboardInput {
        modifiers: manager.keyboard_modifier_state,
        reserved: 0,
        keys: manager.keyboard_keys_state,
    };

    if hid_manager::send_keyboard_input(manager.device_handle, &report).is_ok() {
        VHidResult::Success
    } else {
        VHidResult::Error
    }
}

/// 按下指定的修饰键（如 Ctrl, Shift, Alt）。
/// 修饰键的值可以进行位或运算以实现组合，例如 `LEFT_CTRL | LEFT_SHIFT`。
#[no_mangle]
pub extern "C" fn vhid_modifier_down(modifier: u8) -> VHidResult {
    let mut manager = get_manager().lock().unwrap();
    manager.keyboard_modifier_state |= modifier; // 更新状态

    let report = KeyboardInput {
        modifiers: manager.keyboard_modifier_state,
        reserved: 0,
        keys: manager.keyboard_keys_state,
    };

    if hid_manager::send_keyboard_input(manager.device_handle, &report).is_ok() {
        VHidResult::Success
    } else {
        VHidResult::Error
    }
}

/// 释放指定的修饰键。
#[no_mangle]
pub extern "C" fn vhid_modifier_up(modifier: u8) -> VHidResult {
    let mut manager = get_manager().lock().unwrap();
    manager.keyboard_modifier_state &= !modifier; // 更新状态

    let report = KeyboardInput {
        modifiers: manager.keyboard_modifier_state,
        reserved: 0,
        keys: manager.keyboard_keys_state,
    };

    if hid_manager::send_keyboard_input(manager.device_handle, &report).is_ok() {
        VHidResult::Success
    } else {
        VHidResult::Error
    }
}

/// 点按（按下并立即释放）单个键盘按键。
#[no_mangle]
pub extern "C" fn vhid_key_tap(key: u8) -> VHidResult {
    if vhid_key_down(key) != VHidResult::Success {
        return VHidResult::Error;
    }
    std::thread::sleep(std::time::Duration::from_millis(20));
    vhid_key_up(key)
}

/// 重置所有内部状态，并向设备发送“全部释放”的报告。
/// 这可以用于在操作序列之间或在发生错误后，确保设备处于一个干净的状态。
#[no_mangle]
pub extern "C" fn vhid_reset_state() -> VHidResult {
    let mut manager = get_manager().lock().unwrap();

    // 1. 重置内部状态变量
    manager.mouse_button_state = 0;
    manager.keyboard_modifier_state = 0;
    manager.keyboard_keys_state = [0; 6];

    // 2. 发送一个“全部释放”的鼠标报告
    let mouse_report = MouseInput {
        button: 0,
        x: 0,
        y: 0,
        wheel: 0,
        reserved: 0,
    };
    if hid_manager::send_mouse_input(manager.device_handle, &mouse_report).is_err() {
        // 即使一个失败了，我们仍然尝试重置另一个
    }

    // 3. 发送一个“全部释放”的键盘报告
    let keyboard_report = KeyboardInput {
        modifiers: 0,
        reserved: 0,
        keys: [0; 6],
    };
    if hid_manager::send_keyboard_input(manager.device_handle, &keyboard_report).is_err() {
        return VHidResult::Error; // 如果两个都失败了，就返回错误
    }

    VHidResult::Success
}
