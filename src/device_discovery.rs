use crate::constants::{
    HARDWARE_ID, PRODUCT_ID_VIRTUAL_KEYBOARD, PRODUCT_ID_VIRTUAL_MOUSE, VENDOR_ID_LOGITECH,
};

use crate::types::DeviceIds;
use anyhow::{anyhow, Result};
use core::mem;

// 引入 windows-rs 的模块和类型
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Get_Child, CM_Get_Device_IDW, CM_Get_Sibling, SetupDiDestroyDeviceInfoList,
    SetupDiEnumDeviceInfo, SetupDiGetClassDevsW, SetupDiGetDeviceRegistryPropertyW, CONFIGRET,
    DIGCF_ALLCLASSES, DIGCF_PRESENT, HDEVINFO, MAX_DEVICE_ID_LEN, SPDRP_HARDWAREID,
    SP_DEVINFO_DATA,
};

type DEVINST = u32;
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
            let result: CONFIGRET = unsafe { CM_Get_Device_IDW(child, &mut buffer, 0) };

            if result.0 == 0 {
                // CR_SUCCESS in windows-rs is typically checked by its value
                let instance_id = String::from_utf16_lossy(&buffer)
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
    let device_info_set: HDEVINFO = unsafe {
        SetupDiGetClassDevsW(
            None,
            None,
            None,
            DIGCF_ALLCLASSES | DIGCF_PRESENT,
        )?
    };

    if device_info_set.is_invalid() {
        return Err(anyhow!("获取设备信息集失败 (INVALID_HANDLE_VALUE)。"));
    }

    let mut index = 0;
    loop {
        let mut dev_info_data: SP_DEVINFO_DATA = unsafe { mem::zeroed() };
        dev_info_data.cbSize = mem::size_of::<SP_DEVINFO_DATA>() as u32;

        if unsafe { SetupDiEnumDeviceInfo(device_info_set, index, &mut dev_info_data) }.is_err() {
            break;
        }
        
        // 直接调用 utils 中的函数
        if let Some(hwid) = get_device_hardware_id(device_info_set, &mut dev_info_data) {
            if hwid.to_uppercase().contains(&HARDWARE_ID.to_uppercase()) {
                unsafe { SetupDiDestroyDeviceInfoList(device_info_set) }?;
                return Ok(Some(dev_info_data.DevInst));
            }
        }
        index += 1;
    }

    unsafe { SetupDiDestroyDeviceInfoList(device_info_set) }?;
    Ok(None)
}

/// 获取一个设备的所有子设备
fn get_child_devinsts(parent: DEVINST) -> Result<Vec<DEVINST>> {
    let mut children = Vec::new();
    let mut child_inst: DEVINST = unsafe { mem::zeroed() };

    // 获取第一个子节点
    let mut result: CONFIGRET = unsafe { CM_Get_Child(&mut child_inst, parent, 0) };
    if result.0 != 0 {
        // CR_SUCCESS is 0
        return Ok(children); // 没有子节点
    }
    children.push(child_inst);

    // 循环获取所有兄弟节点
    loop {
        let mut sibling_inst: DEVINST = unsafe { mem::zeroed() };
        result = unsafe { CM_Get_Sibling(&mut sibling_inst, child_inst, 0) };
        if result.0 != 0 {
            break; // 没有更多兄弟节点
        }
        children.push(sibling_inst);
        child_inst = sibling_inst;
    }
    Ok(children)
}

/// 从设备信息集中获取设备的硬件ID
fn get_device_hardware_id(
    dev_info_set: HDEVINFO,
    dev_info_data: &mut SP_DEVINFO_DATA,
) -> Option<String> {
    let mut required_size: u32 = 0;

    // 第一次调用获取大小。windows-rs 的函数在失败时会返回 Error，我们需要捕获它。
    let _ = unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            dev_info_set,
            dev_info_data,
            SPDRP_HARDWAREID,
            None,
            None,
            Some(&mut required_size),
        )
    };

    // 检查是否是因为缓冲区不足而“失败”
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
        // 将 u8 buffer 转换为 u16 slice
        let wide_slice: &[u16] =
            unsafe { std::slice::from_raw_parts(buffer.as_ptr() as *const u16, buffer.len() / 2) };
        Some(
            String::from_utf16_lossy(wide_slice)
                .trim_end_matches('\0')
                .to_string(),
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
