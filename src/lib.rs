//! 罗技虚拟HID设备管理库
//! 提供创建、销毁虚拟设备及发送输入事件的功能

use std::ffi::CString;
use std::ptr;
use std::sync::{Mutex, OnceLock};

use anyhow::Result;

use windows::Win32::Foundation::{CloseHandle, HANDLE};

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
        let device_ids = DeviceIds {
            keyboard_id: self.keyboard_id,
            mouse_id: self.mouse_id,
        };
        if !device_ids.is_empty() {
            // 调用 destroy_hid_devices 来执行销毁逻辑
            if let Err(e) = hid_manager::destroy_hid_devices(&device_ids) {
                println!("[!] 销毁设备时出错: {}", e);
            }
        }

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
#[derive(Debug)]
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

/// 创建或发现虚拟HID设备
/// 返回: 状态码
#[no_mangle]
pub extern "C" fn vhid_create_devices() -> VHidResult {
    let mut manager = get_manager().lock().unwrap();
    match manager.ensure_devices_created() {
        Ok(_) => VHidResult::Success,
        Err(_) => VHidResult::Error,
    }
}

/// 销毁虚拟HID设备
/// 返回: 状态码
#[no_mangle]
pub extern "C" fn vhid_destroy_devices() -> VHidResult {
    let mut manager = get_manager().lock().unwrap();
    let device_ids = DeviceIds {
        keyboard_id: manager.keyboard_id,
        mouse_id: manager.mouse_id,
    };

    if device_ids.is_empty() {
        return VHidResult::Success; // 没有设备可销毁
    }

    if hid_manager::destroy_hid_devices(&device_ids).is_ok() {
        manager.keyboard_id = None;
        manager.mouse_id = None;
        VHidResult::Success
    } else {
        VHidResult::Error
    }
}

/// 移动虚拟鼠标
#[no_mangle]
pub extern "C" fn vhid_move_mouse(input: *const MouseInput) -> VHidResult {
    if input.is_null() {
        return VHidResult::InvalidParameter;
    }
    let manager = get_manager().lock().unwrap();
    if manager.device_handle.is_invalid() {
        return VHidResult::NotInitialized;
    }
    match unsafe { hid_manager::send_mouse_input(manager.device_handle, &*input) } {
        Ok(_) => VHidResult::Success,
        Err(_) => VHidResult::Error,
    }
}

/// 发送键盘输入
#[no_mangle]
pub extern "C" fn vhid_send_keyboard(input: *const KeyboardInput) -> VHidResult {
    if input.is_null() {
        return VHidResult::InvalidParameter;
    }
    let manager = get_manager().lock().unwrap();
    if manager.device_handle.is_invalid() {
        return VHidResult::NotInitialized;
    }
    match unsafe { hid_manager::send_keyboard_input(manager.device_handle, &*input) } {
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

/// 检查设备是否已创建
#[no_mangle]
pub extern "C" fn vhid_devices_created() -> i32 {
    let manager = get_manager().lock().unwrap();
    if manager.keyboard_id.is_some() || manager.mouse_id.is_some() {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn vhid_mouse_move(x: i8, y: i8) -> VHidResult {
    let input = MouseInput {
        button: 0,
        x,
        y,
        wheel: 0,
        reserved: 0,
    };
    vhid_move_mouse(&input)
}

#[no_mangle]
pub extern "C" fn vhid_mouse_click(button: i8) -> VHidResult {
    let input = MouseInput {
        button,
        x: 0,
        y: 0,
        wheel: 0,
        reserved: 0,
    };
    vhid_move_mouse(&input)
}

#[no_mangle]
pub extern "C" fn vhid_mouse_wheel(wheel: i8) -> VHidResult {
    let input = MouseInput {
        button: 0,
        x: 0,
        y: 0,
        wheel,
        reserved: 0,
    };
    vhid_move_mouse(&input)
}

#[no_mangle]
pub extern "C" fn vhid_key_press(key: u8) -> VHidResult {
    let mut keys = [0u8; 6];
    keys[0] = key;
    let input = KeyboardInput {
        modifiers: 0,
        reserved: 0,
        keys,
    };
    vhid_send_keyboard(&input)
}

#[no_mangle]
pub extern "C" fn vhid_key_release() -> VHidResult {
    let input = KeyboardInput {
        modifiers: 0,
        reserved: 0,
        keys: [0u8; 6],
    };
    vhid_send_keyboard(&input)
}
