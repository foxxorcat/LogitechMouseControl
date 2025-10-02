use anyhow::{anyhow, Result};
use std::mem;

use windows::{
    core::{HSTRING, PCWSTR},
    Win32::{
        Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE},
        Storage::FileSystem::{
            CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        },
        System::IO::DeviceIoControl,
    },
};

use crate::device_discovery::DeviceDiscovery;
use crate::types::{DeviceIds, KeyboardInput, MouseInput};
use crate::{constants::*, utils::get_last_error}; // get_last_error 仍然用于 IOCTL 的错误信息

/// 创建虚拟HID设备，并返回发现的设备ID
pub fn create_hid_devices() -> Result<DeviceIds> {
    println!("[*] --- 开始创建并发现虚拟HID设备 ---");
    let bus_handle = open_bus_device()?;

    if let Err(e) = create_single_hid_device(bus_handle, "keyboard") {
        println!("[!] 发送创建键盘请求时出错: {}", e);
    }
    if let Err(e) = create_single_hid_device(bus_handle, "mouse") {
        println!("[!] 发送创建鼠标请求时出错: {}", e);
    }
    unsafe { CloseHandle(bus_handle).ok() }; // .ok() to ignore potential errors on close

    std::thread::sleep(std::time::Duration::from_millis(200));

    let discovered_ids = DeviceDiscovery::discover_devices()?;
    if discovered_ids.is_empty() {
        println!("[!] 警告: 未能发现任何虚拟HID设备。");
    } else {
        println!("\n[成功] 虚拟HID设备已发现并配置。");
    }

    Ok(discovered_ids)
}

/// 根据传入的设备ID销毁虚拟HID设备
pub fn destroy_hid_devices(device_ids: &DeviceIds) -> Result<()> {
    println!("[*] --- 开始销毁虚拟HID设备 ---");
    let bus_handle = open_bus_device()?;

    if let Some(keyboard_id) = device_ids.keyboard_id {
        destroy_single_hid_device(bus_handle, keyboard_id, "keyboard")?;
    }
    if let Some(mouse_id) = device_ids.mouse_id {
        destroy_single_hid_device(bus_handle, mouse_id, "mouse")?;
    }
    unsafe { CloseHandle(bus_handle).ok() };

    println!("\n[成功] 虚拟HID设备清理完毕。");
    Ok(())
}

pub(crate) fn open_bus_device() -> Result<HANDLE> {
    let device_path_hstring = HSTRING::from(BUS_DEVICE_PATH);

    let handle = unsafe {
        CreateFileW(
            PCWSTR::from_raw(device_path_hstring.as_ptr()),
            (GENERIC_READ | GENERIC_WRITE).0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED,
            None,
        )?
    };

    if handle.is_invalid() {
        Err(anyhow!("打开总线设备失败: {}", get_last_error()))
    } else {
        println!("  - 总线设备句柄获取成功。");
        Ok(handle)
    }
}

pub(crate) fn create_single_hid_device(bus_handle: HANDLE, device_type: &str) -> Result<()> {
    println!("[+] 正在构建并发送'创建{}'的IOCTL数据包...", device_type);

    let (buffer_size, pid_vid_part, magic_int1, dev_type_flag, hid_desc) = match device_type {
        "keyboard" => {
            let pid_vid_part = "LGHUBDevice\\VID_046D&PID_C232"
                .encode_utf16()
                .collect::<Vec<u16>>();
            let hid_desc = vec![
                0x05, 0x01, 0x09, 0x06, 0xA1, 0x01, 0x05, 0x07, 0x19, 0xE0, 0x29, 0xE7, 0x15, 0x00,
                0x25, 0x01, 0x75, 0x01, 0x95, 0x08, 0x81, 0x02, 0x95, 0x01, 0x75, 0x08, 0x81, 0x01,
                0x95, 0x05, 0x75, 0x01, 0x05, 0x08, 0x19, 0x01, 0x29, 0x05, 0x91, 0x02, 0x95, 0x01,
                0x75, 0x03, 0x91, 0x01, 0x95, 0x06, 0x75, 0x08, 0x15, 0x00, 0x25, 0x65, 0x05, 0x07,
                0x19, 0x00, 0x29, 0x65, 0x81, 0x00, 0xC0,
            ];
            (
                246,
                pid_vid_part,
                0x3DCDFB93,
                DEVICE_TYPE_KEYBOARD,
                hid_desc,
            )
        }
        "mouse" => {
            let pid_vid_part = "LGHUBDevice\\VID_046D&PID_C231"
                .encode_utf16()
                .collect::<Vec<u16>>();
            let hid_desc = vec![
                0x05, 0x01, 0x09, 0x02, 0xA1, 0x01, 0x09, 0x01, 0xA1, 0x00, 0x05, 0x09, 0x19, 0x01,
                0x29, 0x05, 0x15, 0x00, 0x25, 0x01, 0x95, 0x05, 0x75, 0x01, 0x81, 0x02, 0x95, 0x01,
                0x75, 0x03, 0x81, 0x01, 0x05, 0x01, 0x09, 0x30, 0x09, 0x31, 0x09, 0x38, 0x15, 0x81,
                0x25, 0x7F, 0x75, 0x08, 0x95, 0x03, 0x81, 0x06, 0xC0, 0xC0,
            ];
            (254, pid_vid_part, 0x3DCEFB93, DEVICE_TYPE_MOUSE, hid_desc)
        }
        _ => return Err(anyhow!("无效的设备类型")),
    };

    let mut input_buffer = vec![0u8; buffer_size];
    let mut output_buffer = vec![0u8; buffer_size];
    let mut bytes_returned = 0;

    unsafe {
        std::ptr::copy_nonoverlapping(
            &183u32 as *const _ as *const u8,
            input_buffer.as_mut_ptr(),
            4,
        );
        std::ptr::copy_nonoverlapping(
            &1u32 as *const _ as *const u8,
            input_buffer.as_mut_ptr().add(8),
            4,
        );
        std::ptr::copy_nonoverlapping(
            &62u32 as *const _ as *const u8,
            input_buffer.as_mut_ptr().add(12),
            4,
        );
        let pid_vid_bytes =
            std::slice::from_raw_parts(pid_vid_part.as_ptr() as *const u8, pid_vid_part.len() * 2);
        std::ptr::copy_nonoverlapping(
            pid_vid_bytes.as_ptr(),
            input_buffer.as_mut_ptr().add(16),
            pid_vid_bytes.len().min(128),
        );
        std::ptr::copy_nonoverlapping(
            &magic_int1 as *const _ as *const u8,
            input_buffer.as_mut_ptr().add(144),
            4,
        );
        std::ptr::copy_nonoverlapping(
            &dev_type_flag as *const _ as *const u8,
            input_buffer.as_mut_ptr().add(148),
            4,
        );
        let hid_desc_len = hid_desc.len() as u32;
        std::ptr::copy_nonoverlapping(
            &hid_desc_len as *const _ as *const u8,
            input_buffer.as_mut_ptr().add(178),
            4,
        );
        std::ptr::copy_nonoverlapping(
            hid_desc.as_ptr(),
            input_buffer.as_mut_ptr().add(182),
            hid_desc.len(),
        );
    }

    unsafe {
        DeviceIoControl(
            bus_handle,
            IOCTL_BUS_CREATE_DEVICE,
            Some(input_buffer.as_ptr() as *const _),
            input_buffer.len() as _,
            Some(output_buffer.as_mut_ptr() as *mut _),
            output_buffer.len() as _,
            Some(&mut bytes_returned),
            None,
        )
    }?;

    Ok(())
}

pub(crate) fn destroy_single_hid_device(
    bus_handle: HANDLE,
    device_id: u32,
    device_type: &str,
) -> Result<()> {
    println!(
        "[+] 正在发送'销毁{}'的IOCTL数据包 (ID: {})...",
        device_type, device_id
    );

    let type_flag = if device_type == "keyboard" {
        0u32
    } else {
        1u32
    };
    let mut input_buffer = vec![0u8; 20];

    unsafe {
        std::ptr::copy_nonoverlapping(
            &20u32 as *const _ as *const u8,
            input_buffer.as_mut_ptr(),
            4,
        );
        std::ptr::copy_nonoverlapping(
            &device_id as *const _ as *const u8,
            input_buffer.as_mut_ptr().add(4),
            4,
        );
        std::ptr::copy_nonoverlapping(
            &type_flag as *const _ as *const u8,
            input_buffer.as_mut_ptr().add(8),
            4,
        );
    }

    unsafe {
        DeviceIoControl(
            bus_handle,
            IOCTL_BUS_DESTROY_DEVICE,
            Some(input_buffer.as_ptr() as *const _),
            input_buffer.len() as _,
            None,
            0,
            None,
            None,
        )
    }?;

    println!("  - 成功发送销毁请求。");
    Ok(())
}

/// 发送鼠标输入到虚拟设备
pub fn send_mouse_input(device_handle: HANDLE, input: &MouseInput) -> Result<()> {
    unsafe {
        DeviceIoControl(
            device_handle,
            IOCTL_WRITE_SECONDARY_DEVICE,
            Some(input as *const _ as *const _),
            mem::size_of::<MouseInput>() as _,
            None,
            0,
            None,
            None,
        )
    }?;
    Ok(())
}

/// 发送键盘输入到虚拟设备
pub fn send_keyboard_input(device_handle: HANDLE, input: &KeyboardInput) -> Result<()> {
    unsafe {
        DeviceIoControl(
            device_handle,
            IOCTL_WRITE_PRIMARY_DEVICE,
            Some(input as *const _ as *const _),
            mem::size_of::<KeyboardInput>() as _,
            None,
            0,
            None,
            None,
        )
    }?;
    Ok(())
}

/// 打开可用的虚拟设备句柄
pub fn open_vulnerable_device() -> Result<HANDLE> {
    // 设备路径模板，使用占位符替换序号
    // const DEVICE_PATH_TEMPLATE: &str =;
    // 尝试的设备序号范围
    for number in 1..=3 {
        // 动态生成设备路径
        let path = format!(
            "\\\\.\\ROOT#SYSTEM#000{}#{{1abc05c0-c378-41b9-9cef-df1aba82b015}}",
            number
        );
        let path_hstring = HSTRING::from(&path);

        let handle = unsafe {
            CreateFileW(
                PCWSTR::from_raw(path_hstring.as_ptr()),
                GENERIC_WRITE.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                Default::default(),
                None,
            )
        };

        if let Ok(h) = handle {
            if !h.is_invalid() {
                println!("  - 成功打开虚拟设备: {}", path);
                return Ok(h);
            }
        }
    }

    Err(anyhow!("无法打开任何虚拟设备路径"))
}
