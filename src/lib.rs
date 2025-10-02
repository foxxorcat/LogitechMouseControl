//! 罗技虚拟HID设备管理库
//! 提供创建、销毁虚拟设备及发送输入事件的功能

use std::ffi::CString;
use std::ptr;

mod constants;
mod driver_manager;
mod hid_manager;
mod utils;

use lazy_static::lazy_static;
use std::sync::Mutex;

use anyhow::Result;
use utils::{load_device_ids, save_device_ids, DeviceIds};

use crate::utils::{KeyboardInput, MouseInput};

lazy_static! {
    /// 全局设备句柄管理
    static ref DEVICE_MANAGER: Mutex<Option<DeviceHandleManager>> = Mutex::new(None);
}

/// 设备句柄管理器
struct DeviceHandleManager {
    bus_handle: winapi::um::winnt::HANDLE,
    device_handle: winapi::um::winnt::HANDLE,
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

    fn ensure_devices_created(&mut self) -> Result<()> {
        if self.keyboard_id.is_none() || self.mouse_id.is_none() {
            // 加载已存在的设备ID或创建新设备
            if let Ok(device_ids) = load_device_ids() {
                self.keyboard_id = device_ids.keyboard_id;
                self.mouse_id = device_ids.mouse_id;
            } else {
                // 创建新设备
                self.keyboard_id = Some(hid_manager::create_single_hid_device(
                    self.bus_handle,
                    "keyboard",
                )?);
                self.mouse_id = Some(hid_manager::create_single_hid_device(
                    self.bus_handle,
                    "mouse",
                )?);

                // 保存设备ID
                let device_ids = DeviceIds {
                    keyboard_id: self.keyboard_id,
                    mouse_id: self.mouse_id,
                };
                save_device_ids(&device_ids)?;
            }
        }
        Ok(())
    }
}

impl Drop for DeviceHandleManager {
    fn drop(&mut self) {
        unsafe {
            // 销毁设备
            if let Some(keyboard_id) = self.keyboard_id {
                let _ = hid_manager::destroy_single_hid_device(
                    self.bus_handle,
                    keyboard_id,
                    "keyboard",
                );
            }
            if let Some(mouse_id) = self.mouse_id {
                let _ = hid_manager::destroy_single_hid_device(self.bus_handle, mouse_id, "mouse");
            }

            // 关闭句柄
            if self.device_handle != winapi::um::handleapi::INVALID_HANDLE_VALUE {
                winapi::um::handleapi::CloseHandle(self.device_handle);
            }
            if self.bus_handle != winapi::um::handleapi::INVALID_HANDLE_VALUE {
                winapi::um::handleapi::CloseHandle(self.bus_handle);
            }
        }

        // 删除临时文件
        if std::path::Path::new(crate::constants::TEMP_ID_FILE).exists() {
            let _ = std::fs::remove_file(crate::constants::TEMP_ID_FILE);
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

impl From<Result<()>> for VHidResult {
    fn from(result: Result<()>) -> Self {
        match result {
            Ok(()) => VHidResult::Success,
            Err(_) => VHidResult::Error,
        }
    }
}

/// 初始化虚拟设备系统
/// 返回: 状态码
#[no_mangle]
pub extern "C" fn vhid_initialize() -> VHidResult {
    let mut manager = DEVICE_MANAGER.lock().unwrap();

    if manager.is_some() {
        return VHidResult::Success; // 已经初始化
    }

    match DeviceHandleManager::new() {
        Ok(device_manager) => {
            *manager = Some(device_manager);
            VHidResult::Success
        }
        Err(_) => VHidResult::Error,
    }
}

/// 清理虚拟设备系统
/// 返回: 状态码
#[no_mangle]
pub extern "C" fn vhid_cleanup() -> VHidResult {
    let mut manager = DEVICE_MANAGER.lock().unwrap();
    *manager = None;
    VHidResult::Success
}

/// 创建虚拟HID设备
/// 返回: 状态码
#[no_mangle]
pub extern "C" fn vhid_create_devices() -> VHidResult {
    let mut manager = DEVICE_MANAGER.lock().unwrap();

    if let Some(device_manager) = manager.as_mut() {
        match device_manager.ensure_devices_created() {
            Ok(()) => VHidResult::Success,
            Err(_) => VHidResult::Error,
        }
    } else {
        VHidResult::NotInitialized
    }
}

/// 销毁虚拟HID设备
/// 返回: 状态码
#[no_mangle]
pub extern "C" fn vhid_destroy_devices() -> VHidResult {
    let mut manager = DEVICE_MANAGER.lock().unwrap();

    if let Some(device_manager) = manager.as_mut() {
        device_manager.keyboard_id = None;
        device_manager.mouse_id = None;

        // 删除临时文件
        if std::path::Path::new(crate::constants::TEMP_ID_FILE).exists() {
            let _ = std::fs::remove_file(crate::constants::TEMP_ID_FILE);
        }

        VHidResult::Success
    } else {
        VHidResult::NotInitialized
    }
}

/// 移动虚拟鼠标
/// # 参数:
/// - input: 鼠标输入结构体指针
/// 返回: 状态码
#[no_mangle]
pub extern "C" fn vhid_move_mouse(input: *const MouseInput) -> VHidResult {
    if input.is_null() {
        return VHidResult::InvalidParameter;
    }

    let manager = DEVICE_MANAGER.lock().unwrap();

    if let Some(device_manager) = manager.as_ref() {
        unsafe {
            match hid_manager::send_mouse_input(device_manager.device_handle, &*input) {
                Ok(()) => VHidResult::Success,
                Err(_) => VHidResult::Error,
            }
        }
    } else {
        VHidResult::NotInitialized
    }
}

/// 发送键盘输入
/// # 参数:
/// - input: 键盘输入结构体指针
/// 返回: 状态码
#[no_mangle]
pub extern "C" fn vhid_send_keyboard(input: *const KeyboardInput) -> VHidResult {
    if input.is_null() {
        return VHidResult::InvalidParameter;
    }

    let manager = DEVICE_MANAGER.lock().unwrap();

    if let Some(device_manager) = manager.as_ref() {
        unsafe {
            match hid_manager::send_keyboard_input(device_manager.device_handle, &*input) {
                Ok(()) => VHidResult::Success,
                Err(_) => VHidResult::Error,
            }
        }
    } else {
        VHidResult::NotInitialized
    }
}

/// 获取最后错误信息
/// # 参数:
/// - buffer: 输出缓冲区
/// - size: 缓冲区大小
/// 返回: 实际写入的字符数
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

    copy_size - 1 // 返回字符串长度(不包括null终止符)
}

/// 检查设备是否已创建
/// 返回: 1-已创建, 0-未创建
#[no_mangle]
pub extern "C" fn vhid_devices_created() -> i32 {
    let manager = DEVICE_MANAGER.lock().unwrap();

    if let Some(device_manager) = manager.as_ref() {
        if device_manager.keyboard_id.is_some() || device_manager.mouse_id.is_some() {
            1
        } else {
            0
        }
    } else {
        0
    }
}

/// 便捷函数：移动鼠标相对坐标
/// # 参数:
/// - x: X轴移动量 (-127 到 127)
/// - y: Y轴移动量 (-127 到 127)
/// 返回: 状态码
#[no_mangle]
pub extern "C" fn vhid_mouse_move(x: i8, y: i8) -> VHidResult {
    let input = MouseInput {
        button: 0,
        x,
        y,
        wheel: 0,
        unk1: 0,
    };
    vhid_move_mouse(&input)
}

/// 便捷函数：鼠标点击
/// # 参数:
/// - button: 按钮 (0-无, 1-左键, 2-右键, 3-中键)
/// 返回: 状态码
#[no_mangle]
pub extern "C" fn vhid_mouse_click(button: i8) -> VHidResult {
    let input = MouseInput {
        button,
        x: 0,
        y: 0,
        wheel: 0,
        unk1: 0,
    };
    vhid_move_mouse(&input)
}

/// 便捷函数：鼠标滚轮
/// # 参数:
/// - wheel: 滚轮移动量
/// 返回: 状态码
#[no_mangle]
pub extern "C" fn vhid_mouse_wheel(wheel: i8) -> VHidResult {
    let input = MouseInput {
        button: 0,
        x: 0,
        y: 0,
        wheel,
        unk1: 0,
    };
    vhid_move_mouse(&input)
}

/// 便捷函数：按下单个键盘按键
/// # 参数:
/// - key: 按键HID码
/// 返回: 状态码
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

/// 便捷函数：释放所有按键
/// 返回: 状态码
#[no_mangle]
pub extern "C" fn vhid_key_release() -> VHidResult {
    let input = KeyboardInput {
        modifiers: 0,
        reserved: 0,
        keys: [0u8; 6],
    };
    vhid_send_keyboard(&input)
}
