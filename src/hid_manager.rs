use anyhow::{anyhow, Result};
use winapi::{
    shared::minwindef::DWORD,
    um::fileapi::{CreateFileW, OPEN_EXISTING},
    um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE},
    um::ioapiset::DeviceIoControl,
    um::winnt::{GENERIC_READ, GENERIC_WRITE}, // 修正导入路径
};

use crate::constants::*;
use crate::utils::{
    get_last_error, load_device_ids, save_device_ids, string_to_wide, DeviceIds, KeyboardInput,
    MouseInput,
};

pub fn create_hid_devices() -> Result<()> {
    println!("[*] --- 开始创建虚拟HID设备 ---");

    let bus_handle = open_bus_device()?;
    let mut created_ids = DeviceIds {
        keyboard_id: None,
        mouse_id: None,
    };

    // 创建虚拟键盘
    match create_single_hid_device(bus_handle, "keyboard") {
        Ok(id) => {
            created_ids.keyboard_id = Some(id);
            println!("  - 成功创建键盘设备，ID: {}", id);
        }
        Err(e) => println!("[!] 创建虚拟键盘时出错: {}", e),
    }

    // 创建虚拟鼠标
    match create_single_hid_device(bus_handle, "mouse") {
        Ok(id) => {
            created_ids.mouse_id = Some(id);
            println!("  - 成功创建鼠标设备，ID: {}", id);
        }
        Err(e) => println!("[!] 创建虚拟鼠标时出错: {}", e),
    }

    unsafe { CloseHandle(bus_handle) };

    if created_ids.keyboard_id.is_none() && created_ids.mouse_id.is_none() {
        println!("[!] 未能成功创建任何虚拟HID设备。");
    } else {
        save_device_ids(&created_ids)?;
        println!(
            "\n[成功] 虚拟HID设备创建完毕，ID已保存至 '{}'。",
            TEMP_ID_FILE
        );
    }

    Ok(())
}

pub fn destroy_hid_devices() -> Result<()> {
    println!("[*] --- 开始销毁虚拟HID设备 ---");

    let device_ids = load_device_ids()?;
    let bus_handle = open_bus_device()?;

    if let Some(keyboard_id) = device_ids.keyboard_id {
        destroy_single_hid_device(bus_handle, keyboard_id, "keyboard")?;
    } else {
        println!("[*] 未找到要销毁的键盘设备ID，跳过。");
    }

    if let Some(mouse_id) = device_ids.mouse_id {
        destroy_single_hid_device(bus_handle, mouse_id, "mouse")?;
    } else {
        println!("[*] 未找到要销毁的鼠标设备ID，跳过。");
    }

    unsafe { CloseHandle(bus_handle) };

    if std::path::Path::new(TEMP_ID_FILE).exists() {
        std::fs::remove_file(TEMP_ID_FILE)?;
        println!("[*] 已删除临时ID文件 '{}'。", TEMP_ID_FILE);
    }

    println!("\n[成功] 虚拟HID设备清理完毕。");
    Ok(())
}

pub(crate) fn open_bus_device() -> Result<winapi::um::winnt::HANDLE> {
    let device_path_wide = string_to_wide(BUS_DEVICE_PATH);

    let handle = unsafe {
        CreateFileW(
            device_path_wide.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        )
    };

    if handle == INVALID_HANDLE_VALUE {
        Err(anyhow!("打开总线设备失败: {}", get_last_error()))
    } else {
        println!("  - 总线设备句柄获取成功。");
        Ok(handle)
    }
}

pub(crate) fn create_single_hid_device(
    bus_handle: winapi::um::winnt::HANDLE,
    device_type: &str,
) -> Result<u32> {
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
            (246, pid_vid_part, 0x3DCDFB93, 0u32, hid_desc)
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
            (254, pid_vid_part, 0x3DCEFB93, 1u32, hid_desc)
        }
        _ => return Err(anyhow!("无效的设备类型")),
    };

    let mut input_buffer = vec![0u8; buffer_size];
    let mut output_buffer = vec![0u8; buffer_size];
    let mut bytes_returned = 0;

    // 构建IOCTL输入缓冲区
    unsafe {
        // 填充各个字段
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

        // 复制PID_VID部分
        let pid_vid_bytes =
            std::slice::from_raw_parts(pid_vid_part.as_ptr() as *const u8, pid_vid_part.len() * 2);
        std::ptr::copy_nonoverlapping(
            pid_vid_bytes.as_ptr(),
            input_buffer.as_mut_ptr().add(16),
            pid_vid_bytes.len().min(128), // 确保不越界
        );

        // 复制magic整数和类型标志
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

        // 复制HID描述符长度和内容
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

    let success = unsafe {
        DeviceIoControl(
            bus_handle,
            IOCTL_BUS_CREATE_DEVICE,
            input_buffer.as_mut_ptr() as *mut _,
            input_buffer.len() as DWORD,
            output_buffer.as_mut_ptr() as *mut _,
            output_buffer.len() as DWORD,
            &mut bytes_returned,
            std::ptr::null_mut(),
        )
    };

    if success == 0 {
        return Err(anyhow!("发送IOCTL请求失败: {}", get_last_error()));
    }

    // 从输出缓冲区解析设备ID
    let device_id = unsafe { std::ptr::read(output_buffer.as_ptr().add(4) as *const u32) };

    Ok(device_id)
}

pub(crate)  fn destroy_single_hid_device(
    bus_handle: winapi::um::winnt::HANDLE,
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
    let mut bytes_returned = 0;

    unsafe {
        // 构建销毁请求缓冲区
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

    let success = unsafe {
        DeviceIoControl(
            bus_handle,
            IOCTL_BUS_DESTROY_DEVICE,
            input_buffer.as_mut_ptr() as *mut _,
            input_buffer.len() as DWORD,
            std::ptr::null_mut(),
            0,
            &mut bytes_returned,
            std::ptr::null_mut(),
        )
    };

    if success == 0 {
        return Err(anyhow!(
            "销毁设备 {} 时出错: {}",
            device_id,
            get_last_error()
        ));
    }

    println!("  - 成功发送销毁请求。");
    Ok(())
}

/// 发送鼠标输入到虚拟设备
pub fn send_mouse_input(device_handle: winapi::um::winnt::HANDLE, input: &MouseInput) -> Result<()> {
    let mut bytes_returned = 0;
    let success = unsafe {
        DeviceIoControl(
            device_handle,
            IOCTL_MOVE_MOUSE,
            input as *const _ as *mut _,
            std::mem::size_of::<MouseInput>() as DWORD,
            core::ptr::null_mut(),
            0,
            &mut bytes_returned,
            core::ptr::null_mut(),
        )
    };

    if success == 0 {
        Err(anyhow!("发送鼠标输入失败: {}", get_last_error()))
    } else {
        Ok(())
    }
}

/// 发送键盘输入到虚拟设备
pub fn send_keyboard_input(device_handle: winapi::um::winnt::HANDLE, input: &KeyboardInput) -> Result<()> {
    let mut bytes_returned = 0;
    let success = unsafe {
        DeviceIoControl(
            device_handle,
            IOCTL_SEND_KEYBOARD,
            input as *const _ as *mut _,
            std::mem::size_of::<KeyboardInput>() as DWORD,
            core::ptr::null_mut(),
            0,
            &mut bytes_returned,
            core::ptr::null_mut(),
        )
    };

    if success == 0 {
        Err(anyhow!("发送键盘输入失败: {}", get_last_error()))
    } else {
        Ok(())
    }
}

/// 打开可用的虚拟设备句柄
pub fn open_vulnerable_device() -> Result<winapi::um::winnt::HANDLE> {
    // 尝试多个可能的设备路径
    let device_paths = [
        "\\\\.\\ROOT#SYSTEM#0001#{1abc05c0-c378-41b9-9cef-df1aba82b015}",
        "\\\\.\\ROOT#SYSTEM#0002#{1abc05c0-c378-41b9-9cef-df1aba82b015}",
        "\\\\.\\ROOT#SYSTEM#0003#{1abc05c0-c378-41b9-9cef-df1aba82b015}",
    ];

    for path in &device_paths {
        let path_wide = string_to_wide(path);
        let handle = unsafe {
            CreateFileW(
                path_wide.as_ptr(),
                GENERIC_WRITE,
                0,
                core::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                core::ptr::null_mut(),
            )
        };

        if handle != INVALID_HANDLE_VALUE {
            println!("  - 成功打开虚拟设备: {}", path);
            return Ok(handle);
        }
    }

    Err(anyhow!("无法打开任何虚拟设备路径"))
}
