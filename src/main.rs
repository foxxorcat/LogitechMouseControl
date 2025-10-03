use anyhow::{anyhow, Result};
use std::env;

use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

mod constants;
mod device_discovery;
mod driver_manager;
mod embedded_driver;
mod hid_manager;
mod types;
mod utils;

use crate::device_discovery::DeviceDiscovery;
use crate::embedded_driver::TmpDriverManager;
use crate::hid_manager::{open_vulnerable_device, send_keyboard_input, send_mouse_input};
use crate::types::{KeyboardInput, MouseInput};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];
    match command.as_str() {
        "install" => {
            if !ensure_admin()? {
                return Err(anyhow!("请以管理员身份运行此程序"));
            }
            let tmp_driver_manager = TmpDriverManager::new()?;
            let bus_path = tmp_driver_manager.bus_inf_path()?;
            let hid_path = tmp_driver_manager.hid_inf_path()?;

            driver_manager::install_driver_path(
                bus_path.to_str().unwrap(),
                hid_path.to_str().unwrap(),
            )?;
        }
        "uninstall" => {
            if !ensure_admin()? {
                return Err(anyhow!("请以管理员身份运行此程序"));
            }
            driver_manager::uninstall_driver()?;
        }
        "create-hid" => {
            hid_manager::create_hid_devices()?;
        }
        "destroy-hid" => {
            println!("[*] 正在查找要销毁的设备...");
            let device_ids = DeviceDiscovery::discover_devices()?;
            if device_ids.is_empty() {
                println!("[*] 未发现可销毁的设备。");
            } else {
                hid_manager::destroy_hid_devices(&device_ids)?;
            }
        }
        // --- 新的核心命令 ---
        "mouse-report" => handle_mouse_report(&args)?,
        "keyboard-report" => handle_keyboard_report(&args)?,
        _ => {
            print_usage();
        }
    }
    println!("\n[*] 脚本执行完毕。");
    Ok(())
}

/// 处理原始鼠标报告命令
fn handle_mouse_report(args: &[String]) -> Result<()> {
    // 用法: mouse-report <button> <x> <y> <wheel>
    if args.len() != 6 {
        println!(
            "[!] 用法: {} mouse-report <button> <x> <y> <wheel>",
            args[0]
        );
        println!(
            "    例如: {} mouse-report 1 10 0 0  (左键按下并向右移动10)",
            args[0]
        );
        return Ok(());
    }
    let report = MouseInput {
        button: args[2].parse().map_err(|_| anyhow!("无效的 button 值"))?,
        x: args[3].parse().map_err(|_| anyhow!("无效的 x 值"))?,
        y: args[4].parse().map_err(|_| anyhow!("无效的 y 值"))?,
        wheel: args[5].parse().map_err(|_| anyhow!("无效的 wheel 值"))?,
        reserved: 0,
    };

    let device_handle = open_vulnerable_device()?;
    send_mouse_input(device_handle, &report)?;
    unsafe { CloseHandle(device_handle).ok() };

    println!("[+] 已发送鼠标报告: {:?}", report);
    Ok(())
}

/// 处理原始键盘报告命令
fn handle_keyboard_report(args: &[String]) -> Result<()> {
    // 用法: keyboard-report <modifiers> [key1] [key2] ... [key6]
    if args.len() < 3 || args.len() > 9 {
        println!(
            "[!] 用法: {} keyboard-report <modifiers> [key1] ... [key6]",
            args[0]
        );
        println!(
            "    例如: {} keyboard-report 0 4 5  (同时按下 A 和 B 键)",
            args[0]
        );
        return Ok(());
    }

    let mut keys = [0u8; 6];
    for (i, key_arg) in args.iter().skip(3).enumerate() {
        keys[i] = u8::from_str_radix(key_arg.trim_start_matches("0x"), 16)
            .map_err(|_| anyhow!("无效的 key 值: {}", key_arg))?;
    }

    let report = KeyboardInput {
        modifiers: u8::from_str_radix(args[2].trim_start_matches("0x"), 16)
            .map_err(|_| anyhow!("无效的 modifiers 值: {}", args[2]))?,
        reserved: 0,
        keys,
    };

    let device_handle = open_vulnerable_device()?;
    send_keyboard_input(device_handle, &report)?;
    unsafe { CloseHandle(device_handle).ok() };

    println!("[+] 已发送键盘报告: {:?}", report);
    Ok(())
}

fn print_usage() {
    let exe_name = env::args()
        .next()
        .unwrap_or_else(|| "logi_vhid_manager.exe".to_string());
    println!("\n用法: {} [命令]", exe_name);
    println!("\n核心命令:");
    println!("  install          - 安装虚拟总线驱动和设备");
    println!("  uninstall        - 卸载虚拟总线驱动和设备");
    println!("  create-hid       - 创建虚拟键盘和鼠标设备");
    println!("  destroy-hid      - 销毁已创建的虚拟设备");
    println!("\n输入命令:");
    println!("  mouse-report <button> <x> <y> <wheel> - 发送一个原始鼠标报告");
    println!("    <button>: 0=无, 1=左, 2=右, 3=左+右, 4=中");
    println!("    <x>, <y>, <wheel>: -127 到 127 的值");
    println!("\n  keyboard-report <mods> [k1]..[k6]  - 发送一个原始键盘报告 (所有值为十六进制)");
    println!("    <mods>: 0=无, 1=L-Ctrl, 2=L-Shift, 4=L-Alt, 8=L-Gui, ...");
    println!("    <k1..k6>: HID Usage ID (例如, A=0x04, B=0x05)");
}

/// 检查当前进程是否以管理员权限运行
fn ensure_admin() -> Result<bool> {
    unsafe {
        let mut token_handle: HANDLE = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle)?;

        let mut token_elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut return_length = 0;

        GetTokenInformation(
            token_handle,
            TokenElevation,
            Some(&mut token_elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        )?;

        CloseHandle(token_handle).ok();

        Ok(token_elevation.TokenIsElevated != 0)
    }
}
