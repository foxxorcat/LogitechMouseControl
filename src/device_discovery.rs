use crate::constants::{
    HARDWARE_ID, PRODUCT_ID_VIRTUAL_KEYBOARD, PRODUCT_ID_VIRTUAL_MOUSE, VENDOR_ID_LOGITECH,
};
use crate::types::DeviceIds;
use anyhow::{anyhow, Result};
use std::ffi::OsString;
use std::mem;
use std::os::windows::prelude::OsStringExt;
use std::ptr;
use winapi::um::cfgmgr32::DEVINST;
use winapi::{
    shared::{minwindef::DWORD, ntdef::ULONG},
    um::{
        cfgmgr32::{
            CM_Get_Child, CM_Get_Device_IDW, CM_Get_Sibling, CR_SUCCESS, MAX_DEVICE_ID_LEN,
        },
        setupapi::{
            SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW,
            SetupDiGetDeviceRegistryPropertyW, DIGCF_ALLCLASSES, DIGCF_PRESENT, SPDRP_HARDWAREID,
            SP_DEVINFO_DATA,
        },
    },
};

/// 设备发现管理器
pub struct DeviceDiscovery;

impl DeviceDiscovery {
    /// 通过查找虚拟总线并枚举其子设备来发现设备
    pub fn discover_devices() -> Result<DeviceIds> {
        println!("[*] 开始通过虚拟总线枚举子设备...");

        // 1. 找到我们的虚拟总线设备 (root\LGHUBVirtualBus)
        let bus_devinst = match find_bus_devinst()? {
            Some(inst) => inst,
            None => {
                println!(
                    "[!] 未找到核心总线设备 ('{}')。请确保驱动已安装。",
                    HARDWARE_ID
                );
                // 返回一个空的 DeviceIds，而不是错误，因为这可能是正常状态
                return Ok(DeviceIds::new());
            }
        };

        println!("[*] 成功找到核心总线设备，正在枚举其子设备...");

        // 2. 获取该总线设备的所有子设备
        let child_devinsts = get_child_devinsts(bus_devinst)?;
        if child_devinsts.is_empty() {
            println!("[!] 总线设备下未找到任何子设备。请尝试执行 'create-hid' 命令。");
            return Ok(DeviceIds::new());
        }

        let mut found_ids = DeviceIds::new();

        // 3. 遍历子设备，获取它们的实例ID并解析
        for child in child_devinsts {
            let mut buffer = [0u16; MAX_DEVICE_ID_LEN as usize];
            if unsafe { CM_Get_Device_IDW(child, buffer.as_mut_ptr(), buffer.len() as ULONG, 0) }
                == CR_SUCCESS
            {
                // 将宽字符转换为普通字符串
                let instance_id = OsString::from_wide(&buffer)
                    .into_string()
                    .unwrap_or_default()
                    // 清理尾部的空字符
                    .split('\0')
                    .next()
                    .unwrap_or("")
                    .to_string();

                if instance_id.is_empty() {
                    continue;
                }

                println!("[Debug] 发现子设备实例ID: {}", instance_id);

                if is_virtual_keyboard(&instance_id) {
                    if let Some(id) = parse_device_id(&instance_id) {
                        println!("  - [+] 匹配到虚拟键盘，ID: {}", id);
                        found_ids.keyboard_id = Some(id);
                    }
                } else if is_virtual_mouse(&instance_id) {
                    if let Some(id) = parse_device_id(&instance_id) {
                        println!("  - [+] 匹配到虚拟鼠标，ID: {}", id);
                        found_ids.mouse_id = Some(id);
                    }
                }
            }
        }

        if found_ids.is_empty() {
            println!("[!] 枚举完成，但在子设备中未匹配到已知的虚拟键鼠。");
        }

        Ok(found_ids)
    }
}

/// 查找具有特定硬件ID的设备，并返回其DEVINST
fn find_bus_devinst() -> Result<Option<DEVINST>> {
    let device_info_set = unsafe {
        SetupDiGetClassDevsW(
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            DIGCF_ALLCLASSES | DIGCF_PRESENT,
        )
    };
    if device_info_set.is_null() || device_info_set == winapi::um::handleapi::INVALID_HANDLE_VALUE {
        return Err(anyhow!("无法获取设备信息集。"));
    }

    let mut index = 0;
    loop {
        let mut dev_info_data: SP_DEVINFO_DATA = unsafe { mem::zeroed() };
        dev_info_data.cbSize = mem::size_of::<SP_DEVINFO_DATA>() as u32;

        if unsafe { SetupDiEnumDeviceInfo(device_info_set, index, &mut dev_info_data) } == 0 {
            break;
        }

        if let Some(hwid) = get_device_hardware_id(device_info_set, &mut dev_info_data) {
            // 我们寻找硬件ID为 "root\LGHUBVirtualBus" 的设备
            if hwid.to_uppercase().contains(&HARDWARE_ID.to_uppercase()) {
                unsafe { SetupDiDestroyDeviceInfoList(device_info_set) };
                return Ok(Some(dev_info_data.DevInst));
            }
        }
        index += 1;
    }

    unsafe { SetupDiDestroyDeviceInfoList(device_info_set) };
    Ok(None)
}

/// 获取一个设备的所有子设备
fn get_child_devinsts(parent: DEVINST) -> Result<Vec<DEVINST>> {
    let mut children = Vec::new();
    let mut child_inst = 0;

    // 获取第一个子节点
    if unsafe { CM_Get_Child(&mut child_inst, parent, 0) } != CR_SUCCESS {
        return Ok(children); // 没有子节点
    }
    children.push(child_inst);

    // 循环获取所有兄弟节点
    loop {
        let mut sibling_inst = 0;
        if unsafe { CM_Get_Sibling(&mut sibling_inst, child_inst, 0) } != CR_SUCCESS {
            break; // 没有更多兄弟节点
        }
        children.push(sibling_inst);
        child_inst = sibling_inst;
    }

    Ok(children)
}

/// 从设备信息集中获取设备的硬件ID
fn get_device_hardware_id(
    dev_info_set: winapi::um::setupapi::HDEVINFO,
    dev_info_data: &mut SP_DEVINFO_DATA,
) -> Option<String> {
    let mut required_size: DWORD = 0;

    // 第一次调用获取大小
    unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            dev_info_set,
            dev_info_data,
            SPDRP_HARDWAREID,
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            &mut required_size,
        );
    }

    if required_size == 0 {
        return None;
    }

    let mut buffer = vec![0u16; required_size as usize / 2];
    let success = unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            dev_info_set,
            dev_info_data,
            SPDRP_HARDWAREID,
            ptr::null_mut(),
            buffer.as_mut_ptr() as *mut u8,
            required_size,
            ptr::null_mut(),
        )
    };

    if success != 0 {
        Some(
            OsString::from_wide(&buffer)
                .into_string()
                .unwrap_or_default(),
        )
    } else {
        None
    }
}

/// 从 "LGHUBDevice\VID_...&PID_...\1&...&02" 中解析出最后的数字 ID
fn parse_device_id(instance_id: &str) -> Option<u32> {
    instance_id
        .split('\\')
        .last()
        .and_then(|suffix| suffix.split('&').last())
        .and_then(|id_str| u32::from_str_radix(id_str, 16).ok())
}

fn is_virtual_keyboard(instance_id: &str) -> bool {
    let upper_id = instance_id.to_uppercase();
    let vid = format!("VID_{:04X}", VENDOR_ID_LOGITECH);
    let pid = format!("PID_{:04X}", PRODUCT_ID_VIRTUAL_KEYBOARD);
    upper_id.contains(&vid) && upper_id.contains(&pid)
}

fn is_virtual_mouse(instance_id: &str) -> bool {
    let upper_id = instance_id.to_uppercase();
    let vid = format!("VID_{:04X}", VENDOR_ID_LOGITECH);
    let pid = format!("PID_{:04X}", PRODUCT_ID_VIRTUAL_MOUSE);
    upper_id.contains(&vid) && upper_id.contains(&pid)
}
