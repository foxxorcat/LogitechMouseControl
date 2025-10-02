mod constants;
mod device_discovery;
mod driver_manager;
mod hid_manager;
mod types;
mod utils;

use anyhow::{anyhow, Result};
use std::env;
use std::thread;
use std::time::Duration;
use winapi::um::handleapi::CloseHandle;
use winapi::um::processthreadsapi::GetCurrentProcess;
use winapi::um::processthreadsapi::OpenProcessToken;
use winapi::um::securitybaseapi::GetTokenInformation;
use winapi::um::winnt::TOKEN_ELEVATION;

use crate::device_discovery::DeviceDiscovery;
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
            driver_manager::install_driver()?;
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
            // 先发现，再销毁
            println!("[*] 正在查找要销毁的设备...");
            let device_ids = DeviceDiscovery::discover_devices()?;
            if device_ids.is_empty() {
                println!("[*] 未发现可销毁的设备。");
            } else {
                hid_manager::destroy_hid_devices(&device_ids)?;
            }
        }
        "mouse" => handle_mouse_command(&args)?,
        "keyboard" => handle_keyboard_command(&args)?,
        "demo" => run_demo()?,
        _ => {
            print_usage();
        }
    }

    println!("\n[*] 脚本执行完毕。");
    Ok(())
}

/// 处理鼠标命令
fn handle_mouse_command(args: &[String]) -> Result<()> {
    if args.len() < 3 {
        print_mouse_usage();
        return Ok(());
    }
    // 移除了文件存在性检查
    let sub_command = args[2].to_lowercase();
    let device_handle = open_vulnerable_device()?;

    match sub_command.as_str() {
        "move" => {
            if args.len() < 5 {
                println!("[!] 用法: {} mouse move <x> <y>", args[0]);
                return Ok(());
            }
            let x: i8 = args[3].parse().map_err(|_| anyhow!("无效的X坐标"))?;
            let y: i8 = args[4].parse().map_err(|_| anyhow!("无效的Y坐标"))?;

            let mouse_input = MouseInput {
                button: 0,
                x,
                y,
                wheel: 0,
                reserved: 0,
            };
            send_mouse_input(device_handle, &mouse_input)?;
            println!("[+] 鼠标移动: x={}, y={}", x, y);
        }
        "click" => {
            let button = if args.len() > 3 {
                match args[3].to_lowercase().as_str() {
                    "left" => 1,
                    "right" => 2,
                    "middle" => 3,
                    _ => 1,
                }
            } else {
                1
            };
            let mouse_input = MouseInput {
                button,
                x: 0,
                y: 0,
                wheel: 0,
                reserved: 0,
            };
            send_mouse_input(device_handle, &mouse_input)?;
            println!("[+] 鼠标点击: 按钮={}", button);
        }
        "wheel" => {
            if args.len() < 4 {
                println!("[!] 用法: {} mouse wheel <delta>", args[0]);
                return Ok(());
            }
            let delta: i8 = args[3].parse().map_err(|_| anyhow!("无效的滚轮增量"))?;
            let mouse_input = MouseInput {
                button: 0,
                x: 0,
                y: 0,
                wheel: delta,
                reserved: 0,
            };
            send_mouse_input(device_handle, &mouse_input)?;
            println!("[+] 鼠标滚轮: 增量={}", delta);
        }
        _ => print_mouse_usage(),
    }
    unsafe { CloseHandle(device_handle) };
    Ok(())
}

/// 处理键盘命令
fn handle_keyboard_command(args: &[String]) -> Result<()> {
    if args.len() < 3 {
        print_keyboard_usage();
        return Ok(());
    }
    // 移除了文件存在性检查
    let sub_command = args[2].to_lowercase();
    let device_handle = open_vulnerable_device()?;

    match sub_command.as_str() {
        "press" => {
            if args.len() < 4 {
                println!("[!] 用法: {} keyboard press <key>", args[0]);
                return Ok(());
            }
            let key_name = &args[3];
            let key_code = match key_name.to_lowercase().as_str() {
                "a" => 0x04,
                "b" => 0x05,
                "c" => 0x06,
                "d" => 0x07,
                "e" => 0x08,
                "f" => 0x09,
                "g" => 0x0A,
                "h" => 0x0B,
                "i" => 0x0C,
                "j" => 0x0D,
                "k" => 0x0E,
                "l" => 0x0F,
                "m" => 0x10,
                "n" => 0x11,
                "o" => 0x12,
                "p" => 0x13,
                "q" => 0x14,
                "r" => 0x15,
                "s" => 0x16,
                "t" => 0x17,
                "u" => 0x18,
                "v" => 0x19,
                "w" => 0x1A,
                "x" => 0x1B,
                "y" => 0x1C,
                "z" => 0x1D,
                "1" => 0x1E,
                "2" => 0x1F,
                "3" => 0x20,
                "4" => 0x21,
                "5" => 0x22,
                "6" => 0x23,
                "7" => 0x24,
                "8" => 0x25,
                "9" => 0x26,
                "0" => 0x27,
                "enter" => 0x28,
                "esc" => 0x29,
                "backspace" => 0x2A,
                "tab" => 0x2B,
                "space" => 0x2C,
                _ => return Err(anyhow!("不支持的按键: {}", key_name)),
            };
            let mut keys = [0u8; 6];
            keys[0] = key_code;
            let keyboard_input = KeyboardInput {
                modifiers: 0,
                reserved: 0,
                keys,
            };
            send_keyboard_input(device_handle, &keyboard_input)?;
            println!("[+] 按键按下: {}", key_name);
            thread::sleep(Duration::from_millis(50));
            let release_input = KeyboardInput {
                modifiers: 0,
                reserved: 0,
                keys: [0u8; 6],
            };
            send_keyboard_input(device_handle, &release_input)?;
            println!("[+] 按键释放: {}", key_name);
        }
        "type" => {
            if args.len() < 4 {
                println!("[!] 用法: {} keyboard type <text>", args[0]);
                return Ok(());
            }
            let text = &args[3];
            println!("[+] 正在输入文本: {}", text);
            for ch in text.chars() {
                if let Some(key_code) = char_to_keycode(ch) {
                    let mut keys = [0u8; 6];
                    keys[0] = key_code;
                    let press_input = KeyboardInput {
                        modifiers: 0,
                        reserved: 0,
                        keys,
                    };
                    send_keyboard_input(device_handle, &press_input)?;
                    thread::sleep(Duration::from_millis(20));
                    let release_input = KeyboardInput {
                        modifiers: 0,
                        reserved: 0,
                        keys: [0u8; 6],
                    };
                    send_keyboard_input(device_handle, &release_input)?;
                    thread::sleep(Duration::from_millis(30));
                }
            }
            println!("[+] 文本输入完成");
        }
        _ => print_keyboard_usage(),
    }
    unsafe { CloseHandle(device_handle) };
    Ok(())
}

fn char_to_keycode(ch: char) -> Option<u8> {
    match ch.to_ascii_lowercase() {
        'a'..='z' => Some(ch as u8 - b'a' + 0x04),
        '1'..='9' => Some(ch as u8 - b'1' + 0x1E),
        '0' => Some(0x27),
        ' ' => Some(0x2C),
        _ => None,
    }
}

/// 运行演示
fn run_demo() -> Result<()> {
    println!("[*] 开始虚拟HID设备演示");

    // 检查设备是否存在，如果不存在则创建
    let device_ids = DeviceDiscovery::discover_devices()?;
    if device_ids.is_empty() {
        println!("[*] 未发现虚拟设备，正在创建...");
        hid_manager::create_hid_devices()?;
    }

    let device_handle = open_vulnerable_device()?;
    println!("[*] 演示鼠标移动...");
    for _ in 0..5 {
        let mouse_input = MouseInput {
            button: 0,
            x: 10,
            y: 5,
            wheel: 0,
            reserved: 0,
        };
        send_mouse_input(device_handle, &mouse_input)?;
        thread::sleep(Duration::from_millis(100));
    }

    println!("[*] 演示键盘输入...");
    let text = "Hello from Virtual HID!";
    for ch in text.chars() {
        if let Some(key_code) = char_to_keycode(ch) {
            let mut keys = [0u8; 6];
            keys[0] = key_code;
            let press_input = KeyboardInput {
                modifiers: 0,
                reserved: 0,
                keys,
            };
            send_keyboard_input(device_handle, &press_input)?;
            thread::sleep(Duration::from_millis(50));
            let release_input = KeyboardInput {
                modifiers: 0,
                reserved: 0,
                keys: [0u8; 6],
            };
            send_keyboard_input(device_handle, &release_input)?;
            thread::sleep(Duration::from_millis(50));
        }
    }

    println!("[*] 演示完成");
    unsafe { CloseHandle(device_handle) };
    Ok(())
}

fn print_usage() {
    let exe_name = env::args()
        .next()
        .unwrap_or_else(|| "program.exe".to_string());
    println!("\n用法: {} [命令]", exe_name);
    println!("\n核心命令:");
    println!("  install      - 安装罗技虚拟总线驱动和设备");
    println!("  uninstall    - 卸载罗技虚拟总线驱动和设备");
    println!("  create-hid   - 创建虚拟键盘和鼠标设备");
    println!("  destroy-hid  - 销毁已创建的虚拟键盘和鼠标设备");
    println!("\n输入控制命令:");
    println!("  mouse ...    - 控制鼠标 (move, click, wheel)");
    println!("  keyboard ... - 控制键盘 (press, type)");
    println!("  demo         - 运行演示程序");
}

fn print_mouse_usage() {
    let exe_name = env::args()
        .next()
        .unwrap_or_else(|| "program.exe".to_string());
    println!("\n鼠标命令用法:");
    println!("  {} mouse move <x> <y>    - 移动鼠标相对坐标", exe_name);
    println!("  {} mouse click [left|right|middle] - 鼠标点击", exe_name);
    println!("  {} mouse wheel <delta>   - 鼠标滚轮", exe_name);
}

fn print_keyboard_usage() {
    let exe_name = env::args()
        .next()
        .unwrap_or_else(|| "program.exe".to_string());
    println!("\n键盘命令用法:");
    println!("  {} keyboard press <key>  - 按下并释放单个按键", exe_name);
    println!("  {} keyboard type <text>  - 输入文本", exe_name);
    println!("\n支持的按键: a-z, 0-9, space");
}

fn ensure_admin() -> Result<bool> {
    unsafe {
        let mut token_handle = std::ptr::null_mut();
        if OpenProcessToken(
            GetCurrentProcess(),
            winapi::um::winnt::TOKEN_QUERY,
            &mut token_handle,
        ) == 0
        {
            return Ok(false);
        }

        let mut token_elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut return_length = 0;
        let result = GetTokenInformation(
            token_handle,
            winapi::um::winnt::TokenElevation,
            &mut token_elevation as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        );
        CloseHandle(token_handle);
        Ok(result != 0 && token_elevation.TokenIsElevated != 0)
    }
}
