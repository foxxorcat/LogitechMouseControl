use anyhow::{anyhow, Result};
use winapi::{
    shared::guiddef::GUID,
    shared::minwindef::DWORD,
    shared::winerror::ERROR_INSUFFICIENT_BUFFER,
    um::cfgmgr32::MAX_CLASS_NAME_LEN,
    um::errhandlingapi::GetLastError,
    um::handleapi::INVALID_HANDLE_VALUE,
    um::newdev::{UpdateDriverForPlugAndPlayDevicesW, DIIRFLAG_FORCE_INF},
    um::setupapi::{
        SetupCopyOEMInfW, SetupDiCallClassInstaller, SetupDiCreateDeviceInfoList,
        SetupDiCreateDeviceInfoW, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo,
        SetupDiGetClassDevsW, SetupDiGetDeviceRegistryPropertyW, SetupDiGetINFClassW,
        SetupDiSetDeviceRegistryPropertyW, DICD_GENERATE_ID, DIF_REGISTERDEVICE, DIF_REMOVE,
        DIGCF_ALLCLASSES, SPDRP_HARDWAREID, SP_DEVINFO_DATA,
    },
};

use crate::utils::{get_last_error, string_to_wide, wide_to_string};
use crate::{constants::*, utils::find_inf_file};

pub fn install_driver() -> Result<()> {
    println!("[*] --- 开始驱动安装流程 ---");

    let inf_bus_path = find_inf_file(INF_BUS_FILE)?;

    if check_device_exists()? {
        println!("[*] 设备 '{}' 已存在，跳过创建。", HARDWARE_ID);
    } else {
        println!("[*] 开始创建并安装设备: {}", HARDWARE_ID);
        install_bus_device(&inf_bus_path.to_string_lossy())?;
    }

    // 安装HID驱动
    match find_inf_file(INF_HID_FILE) {
        Ok(inf_hid_path) => {
            println!("[*] 正在安装HID驱动 '{}'...", INF_HID_FILE);
            install_hid_driver(&inf_hid_path.to_string_lossy())?;
        }
        Err(_) => {
            println!(
                "[警告] HID驱动 '{}' 未找到，虚拟设备可能无法工作。",
                INF_HID_FILE
            );
        }
    }

    println!("\n[*] 驱动安装流程执行完毕！");
    Ok(())
}

pub fn uninstall_driver() -> Result<()> {
    println!("[*] 开始卸载流程，目标设备: '{}'...", HARDWARE_ID);

    let dev_info_set = unsafe {
        SetupDiGetClassDevsW(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            DIGCF_ALLCLASSES,
        )
    };

    if dev_info_set == INVALID_HANDLE_VALUE {
        return Err(anyhow!("获取设备列表失败: {}", get_last_error()));
    }

    let mut device_found = false;
    let mut dev_index = 0;

    unsafe {
        while let Some(mut dev_info_data) = get_device_info(dev_info_set, dev_index) {
            dev_index += 1;

            if let Some(hwid) = get_device_hardware_id(dev_info_set, &mut dev_info_data) {
                if hwid.to_lowercase() == HARDWARE_ID.to_lowercase() {
                    device_found = true;
                    println!("[*] 找到匹配的设备，正在执行卸载...");

                    if SetupDiCallClassInstaller(DIF_REMOVE, dev_info_set, &mut dev_info_data) == 0
                    {
                        let error = get_last_error();
                        if !error.contains("ERROR_PNP_REBOOT_REQUIRED") {
                            SetupDiDestroyDeviceInfoList(dev_info_set);
                            return Err(anyhow!("卸载设备失败: {}", error));
                        } else {
                            println!("[*] 设备已标记为卸载，需要重启系统来完成。");
                        }
                    } else {
                        println!("[+] 卸载API调用成功。");
                    }
                    break;
                }
            }
        }

        SetupDiDestroyDeviceInfoList(dev_info_set);
    }

    if !device_found {
        println!("[信息] 未找到需要卸载的设备。");
    } else {
        println!("\n[成功] 卸载操作完成。");
    }

    Ok(())
}

fn install_bus_device(inf_path: &str) -> Result<()> {
    println!("[*] 步骤 1/6: 从INF文件获取设备类GUID...");

    let inf_path_wide = string_to_wide(inf_path);
    let mut class_guid: GUID = unsafe { std::mem::zeroed() };
    let mut class_name = vec![0u16; MAX_CLASS_NAME_LEN];
    let mut required_size = 0;

    let success = unsafe {
        SetupDiGetINFClassW(
            inf_path_wide.as_ptr(),
            &mut class_guid,
            class_name.as_mut_ptr(),
            class_name.len() as DWORD,
            &mut required_size,
        )
    };

    if success == 0 {
        return Err(anyhow!("获取INF设备类失败: {}", get_last_error()));
    }

    let class_name_str = wide_to_string(&class_name);
    println!("  - 成功获取设备类: {}", class_name_str);

    println!("[*] 步骤 2/6: 正在创建设备信息列表...");
    let dev_info_set =
        unsafe { SetupDiCreateDeviceInfoList(&class_guid as *const GUID, std::ptr::null_mut()) };

    if dev_info_set == INVALID_HANDLE_VALUE {
        return Err(anyhow!("创建设备信息列表失败: {}", get_last_error()));
    }

    let mut dev_info_data = SP_DEVINFO_DATA {
        cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as DWORD,
        ClassGuid: unsafe { std::mem::zeroed() },
        DevInst: 0,
        Reserved: 0,
    };

    let device_name_wide = string_to_wide(DEVICE_NAME);

    println!("[*] 步骤 3/6: 正在创建设备实例...");
    let success = unsafe {
        SetupDiCreateDeviceInfoW(
            dev_info_set,
            device_name_wide.as_ptr(),
            &class_guid,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            DICD_GENERATE_ID,
            &mut dev_info_data,
        )
    };

    if success == 0 {
        unsafe { SetupDiDestroyDeviceInfoList(dev_info_set) };
        return Err(anyhow!("创建设备信息失败: {}", get_last_error()));
    }

    println!("[*] 步骤 4/6: 正在设置Hardware ID: {}...", HARDWARE_ID);
    let hwid_wide = string_to_wide(&(HARDWARE_ID.to_string() + "\0"));

    let success = unsafe {
        SetupDiSetDeviceRegistryPropertyW(
            dev_info_set,
            &mut dev_info_data,
            SPDRP_HARDWAREID,
            hwid_wide.as_ptr() as *const u8,
            (hwid_wide.len() * 2) as DWORD, // 宽字符字节数
        )
    };

    if success == 0 {
        unsafe { SetupDiDestroyDeviceInfoList(dev_info_set) };
        return Err(anyhow!("设置Hardware ID失败: {}", get_last_error()));
    }

    println!("[*] 步骤 5/6: 正在注册设备实例...");
    let success =
        unsafe { SetupDiCallClassInstaller(DIF_REGISTERDEVICE, dev_info_set, &mut dev_info_data) };

    if success == 0 {
        unsafe { SetupDiDestroyDeviceInfoList(dev_info_set) };
        return Err(anyhow!("注册设备失败: {}", get_last_error()));
    }

    println!("[*] 步骤 6/6: 正在为新设备安装驱动...");
    let hwid_wide = string_to_wide(HARDWARE_ID);
    let inf_path_wide = string_to_wide(inf_path);
    let mut reboot_required = 0;

    // 使用 UpdateDriverForPlugAndPlayDevicesW 来安装驱动
    let success = unsafe {
        UpdateDriverForPlugAndPlayDevicesW(
            std::ptr::null_mut(), // 无父窗口
            hwid_wide.as_ptr(),
            inf_path_wide.as_ptr(),
            DIIRFLAG_FORCE_INF, // 强制使用指定的INF文件
            &mut reboot_required,
        )
    };

    if success == 0 {
        unsafe { SetupDiDestroyDeviceInfoList(dev_info_set) };
        return Err(anyhow!("安装驱动失败: {}", get_last_error()));
    }

    unsafe { SetupDiDestroyDeviceInfoList(dev_info_set) };
    println!("[+] 核心总线设备创建并安装成功！");

    if reboot_required != 0 {
        println!("[注意] 系统提示需要重启。");
    }

    Ok(())
}

fn install_hid_driver(inf_path: &str) -> Result<()> {
    let inf_path_wide = string_to_wide(inf_path);

    let success = unsafe {
        SetupCopyOEMInfW(
            inf_path_wide.as_ptr(),
            std::ptr::null_mut(),
            1,
            0,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };

    if success != 0 {
        println!("  - [成功] 驱动 '{}' 安装/更新成功。", INF_HID_FILE);
    } else {
        let error = get_last_error();
        if error.contains("ERROR_FILE_EXISTS") {
            println!("  - [信息] 驱动 '{}' 已存在于驱动仓库中。", INF_HID_FILE);
        } else {
            return Err(anyhow!("安装HID驱动失败: {}", error));
        }
    }

    Ok(())
}

fn check_device_exists() -> Result<bool> {
    let dev_info_set = unsafe {
        SetupDiGetClassDevsW(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            DIGCF_ALLCLASSES,
        )
    };

    if dev_info_set == INVALID_HANDLE_VALUE {
        return Ok(false);
    }

    let mut dev_index = 0;
    let mut found = false;

    unsafe {
        while let Some(mut dev_info_data) = get_device_info(dev_info_set, dev_index) {
            dev_index += 1;

            if let Some(hwid) = get_device_hardware_id(dev_info_set, &mut dev_info_data) {
                if hwid.to_lowercase() == HARDWARE_ID.to_lowercase() {
                    found = true;
                    break;
                }
            }
        }

        SetupDiDestroyDeviceInfoList(dev_info_set);
    }

    Ok(found)
}

fn get_device_info(
    dev_info_set: winapi::um::setupapi::HDEVINFO,
    index: DWORD,
) -> Option<SP_DEVINFO_DATA> {
    let mut dev_info_data = SP_DEVINFO_DATA {
        cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as DWORD,
        ClassGuid: unsafe { std::mem::zeroed() },
        DevInst: 0,
        Reserved: 0,
    };

    let success = unsafe { SetupDiEnumDeviceInfo(dev_info_set, index, &mut dev_info_data) };

    if success != 0 {
        Some(dev_info_data)
    } else {
        None
    }
}

fn get_device_hardware_id(
    dev_info_set: winapi::um::setupapi::HDEVINFO,
    dev_info_data: &mut SP_DEVINFO_DATA,
) -> Option<String> {
    let mut required_size = 0;

    // 第一次调用获取需要的缓冲区大小
    unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            dev_info_set,
            dev_info_data,
            SPDRP_HARDWAREID,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            &mut required_size,
        )
    };

    if unsafe { GetLastError() } != ERROR_INSUFFICIENT_BUFFER {
        return None;
    }

    let mut buffer = vec![0u8; required_size as usize];
    let success = unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            dev_info_set,
            dev_info_data,
            SPDRP_HARDWAREID,
            std::ptr::null_mut(),
            buffer.as_mut_ptr(),
            buffer.len() as DWORD,
            &mut required_size,
        )
    };

    if success != 0 {
        // 将宽字符缓冲区转换为字符串
        let wide_slice = unsafe {
            std::slice::from_raw_parts(buffer.as_ptr() as *const u16, required_size as usize / 2)
        };
        Some(wide_to_string(wide_slice))
    } else {
        None
    }
}
