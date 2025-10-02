use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::winbase::FormatMessageW;
use winapi::um::winnt::LPWSTR;

/// 鼠标输入结构体
#[repr(C)]
pub struct MouseInput {
    pub button: i8,   // 按钮状态
    pub x: i8,        // X轴移动 (-127 到 127)
    pub y: i8,        // Y轴移动 (-127 到 127)
    pub wheel: i8,    // 滚轮移动
    pub unk1: i8,     // 保留字段
}

/// 键盘输入结构体
#[repr(C)]
pub struct KeyboardInput {
    pub modifiers: u8,     // 修饰键 (Ctrl, Alt, Shift等)
    pub reserved: u8,      // 保留字段
    pub keys: [u8; 6],     // 按键码数组 (最多6个按键)
}

/// 设备ID存储结构
#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceIds {
    pub keyboard_id: Option<u32>,
    pub mouse_id: Option<u32>,
}

/// 查找INF文件，支持子目录搜索
pub fn find_inf_file(filename: &str) -> Result<PathBuf> {
    let current_dir = std::env::current_dir()?;
    
    // 首先检查当前目录
    let current_path = current_dir.join(filename);
    if current_path.exists() {
        return Ok(current_path);
    }
    
    // 递归搜索子目录
    for entry in walkdir::WalkDir::new(&current_dir)
        .max_depth(3) // 限制搜索深度
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() && entry.file_name() == filename {
            return Ok(entry.path().to_path_buf());
        }
    }
    
    Err(anyhow!("未找到INF文件: {} (已搜索当前目录及子目录)", filename))
}

/// 获取Windows最后错误信息
pub fn get_last_error() -> String {
    unsafe {
        let error_code = GetLastError();
        let mut buffer: [u16; 256] = [0; 256];

        let length = FormatMessageW(
            winapi::um::winbase::FORMAT_MESSAGE_FROM_SYSTEM
                | winapi::um::winbase::FORMAT_MESSAGE_IGNORE_INSERTS,
            std::ptr::null_mut(),
            error_code,
            0,
            buffer.as_mut_ptr() as LPWSTR,
            buffer.len() as u32,
            std::ptr::null_mut(),
        );

        if length > 0 {
            let error_msg = String::from_utf16_lossy(&buffer[..length as usize]);
            format!("[WinError {}] {}", error_code, error_msg.trim())
        } else {
            format!("[WinError {}] Unknown error", error_code)
        }
    }
}

/// 加载设备ID文件
pub fn load_device_ids() -> Result<DeviceIds> {
    if !Path::new(super::constants::TEMP_ID_FILE).exists() {
        return Err(anyhow!(
            "找不到设备ID文件 '{}'。请先执行 'create-hid' 命令。",
            super::constants::TEMP_ID_FILE
        ));
    }

    let content = fs::read_to_string(super::constants::TEMP_ID_FILE)?;
    let device_ids: DeviceIds = serde_json::from_str(&content)?;
    Ok(device_ids)
}

/// 保存设备ID到文件
pub fn save_device_ids(device_ids: &DeviceIds) -> Result<()> {
    let content = serde_json::to_string_pretty(device_ids)?;
    fs::write(super::constants::TEMP_ID_FILE, content)?;
    Ok(())
}

/// 将字符串转换为宽字符串
pub fn string_to_wide(string: &str) -> Vec<u16> {
    string.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 将宽字符串转换为普通字符串
pub fn wide_to_string(wide: &[u16]) -> String {
    let len = wide.iter().position(|&x| x == 0).unwrap_or(wide.len());
    String::from_utf16_lossy(&wide[..len])
}