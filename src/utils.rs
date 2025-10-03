use anyhow::{anyhow, Result};
use std::path::PathBuf;
use windows::{
    core::GUID,
    Win32::Devices::DeviceAndDriverInstallation::{
        SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
        SetupDiGetDeviceInterfaceDetailW, SetupDiGetDeviceRegistryPropertyW, DIGCF_DEVICEINTERFACE,
        DIGCF_PRESENT, HDEVINFO, SPDRP_HARDWAREID, SP_DEVICE_INTERFACE_DATA,
        SP_DEVICE_INTERFACE_DETAIL_DATA_W, SP_DEVINFO_DATA,
    },
};

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
        .max_depth(5) // 限制搜索深度
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() && entry.file_name() == filename {
            return Ok(entry.path().to_path_buf());
        }
    }

    Err(anyhow!(
        "未找到INF文件: {} (已搜索当前目录及子目录)",
        filename
    ))
}

/// 获取Windows最后错误信息的现代化实现
/// 注意：这个函数主要用于那些不返回 Result 的旧式 IOCTL 调用，
/// 对于大多数 windows-rs 函数，直接处理返回的 Error 会更好。
pub fn get_last_error() -> String {
    // Error::from_win32() 会自动调用 GetLastError() 并获取对应的错误信息
    let error = windows::core::Error::from_thread();
    format!("[WinError {}] {}", error.code().0, error.message())
}

pub fn get_current_exe() -> String {
    std::env::current_exe()
        .map_err(|_| ())
        .and_then(|path| {
            path.file_name()
                .map(|os_str| os_str.to_os_string())
                .ok_or(())
        })
        .and_then(|os_string| os_string.into_string().map_err(|_| ()))
        .unwrap_or_else(|_| "logi_vhid_manager.exe".to_string())
}

/// 通过设备信息集和设备信息数据获取设备的硬件ID。
/// 这是一个通用的辅助函数，用于从一个已发现的设备中提取其硬件ID。
pub fn get_device_hardware_id(
    dev_info_set: HDEVINFO,
    dev_info_data: &mut SP_DEVINFO_DATA,
) -> Option<String> {
    let mut required_size = 0;

    // 第一次调用以获取所需缓冲区的大小
    unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            dev_info_set,
            dev_info_data,
            SPDRP_HARDWAREID,
            None,
            None,
            Some(&mut required_size),
        )
        .ok(); // 我们预期这里会因为缓冲区不足而失败
    }

    if required_size == 0 {
        return None;
    }

    let mut buffer = vec![0u8; required_size as usize];

    // 第二次调用以获取实际数据
    if unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            dev_info_set,
            dev_info_data,
            SPDRP_HARDWAREID,
            None,
            Some(&mut buffer),
            Some(&mut required_size),
        )
    }
    .is_ok()
    {
        // 将返回的缓冲区（一系列宽字符）转换成Rust字符串
        let wide_slice: &[u16] =
            unsafe { std::slice::from_raw_parts(buffer.as_ptr() as *const u16, buffer.len() / 2) };
        // 硬件ID是一个多字符串（以两个NULL结尾），我们只需要第一个
        Some(
            String::from_utf16_lossy(wide_slice)
                .split('\0')
                .next()
                .unwrap_or("")
                .to_string(),
        )
    } else {
        None
    }
}

/// 通过设备接口GUID动态查找设备的路径。
///
/// # Arguments
/// * `interface_guid` - 要查找的设备的接口GUID。
///
/// # Returns
/// `Ok(String)` 包含找到的第一个匹配设备的路径。
/// `Err` 如果未找到设备或发生API错误。
pub fn find_device_path_by_interface_guid(interface_guid: GUID) -> Result<String> {
    unsafe {
        // 1. 获取所有实现了指定接口GUID的、当前存在的设备
        let dev_info_set = SetupDiGetClassDevsW(
            Some(&interface_guid),
            None,
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )?;

        if dev_info_set.is_invalid() {
            return Err(anyhow!("无法获取设备信息集 (SetupDiGetClassDevsW)"));
        }

        // 2. 枚举设备接口
        let mut dev_interface_data: SP_DEVICE_INTERFACE_DATA = std::mem::zeroed();
        dev_interface_data.cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;

        if SetupDiEnumDeviceInterfaces(
            dev_info_set,
            None,
            &interface_guid,
            0,
            &mut dev_interface_data,
        )
        .is_err()
        {
            SetupDiDestroyDeviceInfoList(dev_info_set).ok();
            return Err(anyhow!("未找到活动的设备接口 (GUID: {:?})", interface_guid));
        }

        // 3. 获取设备接口的详细信息（包含设备路径）
        let mut required_size = 0;
        SetupDiGetDeviceInterfaceDetailW(
            dev_info_set,
            &dev_interface_data,
            None,
            0,
            Some(&mut required_size),
            None,
        )
        .ok();
        if required_size == 0 {
            SetupDiDestroyDeviceInfoList(dev_info_set).ok();
            return Err(anyhow!("无法获取设备接口详细信息的所需大小。"));
        }

        let mut detail_data_buffer = vec![0u8; required_size as usize];
        let detail_data = detail_data_buffer.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
        (*detail_data).cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;

        if SetupDiGetDeviceInterfaceDetailW(
            dev_info_set,
            &dev_interface_data,
            Some(detail_data),
            required_size,
            None,
            None,
        )
        .is_err()
        {
            SetupDiDestroyDeviceInfoList(dev_info_set).ok();
            return Err(anyhow!("获取设备接口详细信息失败。"));
        }

        // 从结构体中获取设备路径的宽字符切片
        let device_path_ptr = &(*detail_data).DevicePath as *const u16;

        // 计算字符串实际长度
        let mut len = 0;
        while *device_path_ptr.add(len) != 0 {
            len += 1;
        }
        let wide_slice = std::slice::from_raw_parts(device_path_ptr, len);
        let device_path = String::from_utf16(wide_slice)?;

        // 清理资源并返回路径
        SetupDiDestroyDeviceInfoList(dev_info_set).ok();
        Ok(device_path)
    }
}
