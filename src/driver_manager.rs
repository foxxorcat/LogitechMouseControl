use anyhow::{anyhow, Result};

use windows::{
    core::{GUID, HSTRING, PCWSTR},
    Win32::{
        Devices::DeviceAndDriverInstallation::{
            SetupCopyOEMInfW, SetupDiCallClassInstaller, SetupDiCreateDeviceInfoList,
            SetupDiCreateDeviceInfoW, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo,
            SetupDiGetClassDevsW, SetupDiGetDeviceRegistryPropertyW, SetupDiGetINFClassW,
            SetupDiSetDeviceRegistryPropertyW, UpdateDriverForPlugAndPlayDevicesW,
            DICD_GENERATE_ID, DIF_REGISTERDEVICE, DIF_REMOVE, DIGCF_ALLCLASSES, HDEVINFO,
            INSTALLFLAG_FORCE, MAX_CLASS_NAME_LEN, SPDRP_HARDWAREID, SPOST_NONE,
            SP_COPY_NOOVERWRITE, SP_DEVINFO_DATA,
        },
        Foundation::{ERROR_FILE_EXISTS, FALSE},
    },
};

use crate::{constants::*, utils::find_inf_file};

pub fn install_driver() -> Result<()> {
    println!("[*] --- 开始驱动安装流程 ---");

    let inf_bus_path = find_inf_file(INF_BUS_FILE)?;
    println!("[*] 找到总线驱动: {}", inf_bus_path.display());

    if check_device_exists()? {
        println!("[*] 设备 '{}' 已存在，跳过创建。", HARDWARE_ID);
    } else {
        println!("[*] 开始创建并安装设备: {}", HARDWARE_ID);
        install_bus_device(&inf_bus_path.to_string_lossy())?;
    }

    if let Ok(inf_hid_path) = find_inf_file(INF_HID_FILE) {
        println!("[*] 找到HID驱动: {}", inf_hid_path.display());
        println!("[*] 正在安装HID驱动 '{}'...", INF_HID_FILE);
        install_hid_driver(&inf_hid_path.to_string_lossy())?;
    } else {
        println!(
            "[警告] HID驱动 '{}' 未找到，虚拟设备可能无法工作。",
            INF_HID_FILE
        );
    }

    println!("\n[*] 驱动安装流程执行完毕！");
    Ok(())
}

pub fn uninstall_driver() -> Result<()> {
    println!("[*] 开始卸载流程，目标设备: '{}'...", HARDWARE_ID);

    let dev_info_set: HDEVINFO =
        unsafe { SetupDiGetClassDevsW(None, None, None, DIGCF_ALLCLASSES) }?;

    if dev_info_set.is_invalid() {
        return Err(anyhow!("获取设备列表失败。"));
    }

    let mut device_found = false;
    let mut dev_index = 0;

    loop {
        let mut dev_info_data: SP_DEVINFO_DATA = unsafe { std::mem::zeroed() };
        dev_info_data.cbSize = std::mem::size_of::<SP_DEVINFO_DATA>() as u32;

        if unsafe { SetupDiEnumDeviceInfo(dev_info_set, dev_index, &mut dev_info_data) }.is_err() {
            break;
        }

        if let Some(hwid) = get_device_hardware_id(dev_info_set, &mut dev_info_data) {
            if hwid.to_uppercase().contains(&HARDWARE_ID.to_uppercase()) {
                device_found = true;
                println!("[*] 找到匹配的设备，正在执行卸载...");

                if let Err(err) = unsafe {
                    SetupDiCallClassInstaller(DIF_REMOVE, dev_info_set, Some(&mut dev_info_data))
                } {
                    // PNP_REBOOT_REQUIRED is not a real error, so we can ignore it.
                    // For other errors, we should report them.
                    let error = err.message();
                    if !error.contains("ERROR_PNP_REBOOT_REQUIRED") {
                        unsafe { SetupDiDestroyDeviceInfoList(dev_info_set).ok() };
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
        dev_index += 1;
    }

    unsafe { SetupDiDestroyDeviceInfoList(dev_info_set)? };

    if !device_found {
        println!("[信息] 未找到需要卸载的设备。");
    } else {
        println!("\n[成功] 卸载操作完成。");
    }

    Ok(())
}

fn install_bus_device(inf_path: &str) -> Result<()> {
    println!("[*] 步骤 1/6: 从INF文件获取设备类GUID...");
    let inf_path_hstring = HSTRING::from(inf_path);
    let inf_path_pcwstr = PCWSTR::from_raw(inf_path_hstring.as_ptr());

    let mut class_guid: GUID = GUID::default();
    let mut class_name = [0u16; MAX_CLASS_NAME_LEN as usize];

    unsafe { SetupDiGetINFClassW(inf_path_pcwstr, &mut class_guid, &mut class_name, None)? };

    let class_name_str = String::from_utf16_lossy(&class_name);
    println!(
        "  - 成功获取设备类: {}",
        class_name_str.trim_end_matches('\0')
    );

    println!("[*] 步骤 2/6: 正在创建设备信息列表...");
    let dev_info_set = unsafe { SetupDiCreateDeviceInfoList(Some(&class_guid), None) }?;
    if dev_info_set.is_invalid() {
        return Err(anyhow!("创建设备信息列表失败"));
    }

    let mut dev_info_data: SP_DEVINFO_DATA = unsafe { std::mem::zeroed() };
    dev_info_data.cbSize = std::mem::size_of::<SP_DEVINFO_DATA>() as u32;

    let device_name_hstring = HSTRING::from(DEVICE_NAME);

    println!("[*] 步骤 3/6: 正在创建设备实例...");
    unsafe {
        SetupDiCreateDeviceInfoW(
            dev_info_set,
            PCWSTR::from_raw(device_name_hstring.as_ptr()),
            &class_guid,
            None,
            None,
            DICD_GENERATE_ID,
            Some(&mut dev_info_data),
        )?
    };

    println!("[*] 步骤 4/6: 正在设置Hardware ID: {}...", HARDWARE_ID);
    let hwid_str = format!("{}\0\0", HARDWARE_ID);
    let hwid_wide: Vec<u16> = hwid_str.encode_utf16().collect();

    let hwid_bytes: &[u8] =
        unsafe { std::slice::from_raw_parts(hwid_wide.as_ptr() as *const u8, hwid_wide.len() * 2) };

    unsafe {
        SetupDiSetDeviceRegistryPropertyW(
            dev_info_set,
            &mut dev_info_data,
            SPDRP_HARDWAREID,
            Some(hwid_bytes),
        )?
    };

    println!("[*] 步骤 5/6: 正在注册设备实例...");
    unsafe {
        SetupDiCallClassInstaller(DIF_REGISTERDEVICE, dev_info_set, Some(&mut dev_info_data))?
    };

    println!("[*] 步骤 6/6: 正在为新设备安装驱动...");
    let hwid_hstring = HSTRING::from(HARDWARE_ID);
    let mut reboot_required = FALSE;

    unsafe {
        UpdateDriverForPlugAndPlayDevicesW(
            None,
            PCWSTR::from_raw(hwid_hstring.as_ptr()),
            inf_path_pcwstr,
            INSTALLFLAG_FORCE,
            Some(&mut reboot_required),
        )?
    };

    unsafe { SetupDiDestroyDeviceInfoList(dev_info_set)? };
    println!("[+] 核心总线设备创建并安装成功！");

    if reboot_required.as_bool() {
        println!("[注意] 系统提示需要重启。");
    }

    Ok(())
}

fn install_hid_driver(inf_path: &str) -> Result<()> {
    let inf_path_hstring = HSTRING::from(inf_path);

    let result = unsafe {
        SetupCopyOEMInfW(
            PCWSTR::from_raw(inf_path_hstring.as_ptr()),
            None,
            SPOST_NONE,          // SPOST_NONE
            SP_COPY_NOOVERWRITE, // SP_COPY_NOOVERWRITE
            None,
            None,
            None,
        )
    };

    if let Err(e) = result {
        if e.code() == ERROR_FILE_EXISTS.to_hresult() {
            println!("  - [信息] 驱动 '{}' 已存在于驱动仓库中。", INF_HID_FILE);
        } else {
            return Err(anyhow!("安装HID驱动失败: {}", e));
        }
    } else {
        println!("  - [成功] 驱动 '{}' 安装/更新成功。", INF_HID_FILE);
    }

    Ok(())
}

fn check_device_exists() -> Result<bool> {
    let dev_info_set: HDEVINFO =
        unsafe { SetupDiGetClassDevsW(None, None, None, DIGCF_ALLCLASSES) }?;
    if dev_info_set.is_invalid() {
        return Ok(false);
    }

    let mut dev_index = 0;
    let mut found = false;
    loop {
        let mut dev_info_data: SP_DEVINFO_DATA = unsafe { std::mem::zeroed() };
        dev_info_data.cbSize = std::mem::size_of::<SP_DEVINFO_DATA>() as u32;

        if unsafe { SetupDiEnumDeviceInfo(dev_info_set, dev_index, &mut dev_info_data) }.is_err() {
            break;
        }

        if let Some(hwid) = get_device_hardware_id(dev_info_set, &mut dev_info_data) {
            if hwid.to_uppercase().contains(&HARDWARE_ID.to_uppercase()) {
                found = true;
                break;
            }
        }
        dev_index += 1;
    }

    unsafe { SetupDiDestroyDeviceInfoList(dev_info_set)? };
    Ok(found)
}

fn get_device_hardware_id(
    dev_info_set: HDEVINFO,
    dev_info_data: &mut SP_DEVINFO_DATA,
) -> Option<String> {
    let mut required_size = 0;

    // First call to get the size
    unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            dev_info_set,
            dev_info_data,
            SPDRP_HARDWAREID,
            None,
            None,
            Some(&mut required_size),
        )
        .ok(); // We expect this to fail with ERROR_INSUFFICIENT_BUFFER
    }

    if required_size == 0 {
        return None;
    }

    let mut buffer = vec![0u8; required_size as usize];

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
        let wide_slice: &[u16] =
            unsafe { std::slice::from_raw_parts(buffer.as_ptr() as *const u16, buffer.len() / 2) };
        // The buffer is a multi-string, so we just take the first one.
        Some(
            String::from_utf16_lossy(&wide_slice)
                .split('\0')
                .next()
                .unwrap_or("")
                .to_string(),
        )
    } else {
        None
    }
}
