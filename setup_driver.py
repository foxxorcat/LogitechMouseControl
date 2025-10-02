import ctypes
import sys
import os
from ctypes import wintypes

# --- 全局常量和API定义 ---
setupapi = ctypes.windll.setupapi
newdev = ctypes.windll.newdev
kernel32 = ctypes.windll.kernel32

# --- WinAPI 常量 ---
INF_FILENAME = "logi_joy_bus_enum.inf"
HARDWARE_ID = "root\\LGHUBVirtualBus"
DEVICE_NAME = "System"  # 根据日志，创建设备时使用 "System"

INVALID_HANDLE_VALUE = ctypes.c_void_p(-1).value
DIF_REGISTERDEVICE = 0x00000019
DIF_REMOVE = 0x00000005
DIIRFLAG_FORCE_INF = 0x00000002  # INSTALLFLAG_FORCE
SPDRP_HARDWAREID = 0x00000001
DICD_GENERATE_ID = 0x00000001
DIGCF_PRESENT = 0x00000002
DIGCF_ALLCLASSES = 0x00000004
MAX_CLASS_NAME_LEN = 32
MAX_PATH = 260
ERROR_INSUFFICIENT_BUFFER = 122

# --- 结构体与GUID定义 ---
class GUID(ctypes.Structure):
    _fields_ = [
        ("Data1", wintypes.DWORD),
        ("Data2", wintypes.WORD),
        ("Data3", wintypes.WORD),
        ("Data4", (ctypes.c_ubyte * 8))
    ]

class SP_DEVINFO_DATA(ctypes.Structure):
    _fields_ = [
        ("cbSize", wintypes.DWORD),
        ("ClassGuid", GUID),
        ("DevInst", wintypes.DWORD), # DEVINST
        ("Reserved", ctypes.POINTER(wintypes.ULONG))
    ]

# --- API 函数原型定义 ---
setupapi.SetupDiGetINFClassW.restype = wintypes.BOOL
setupapi.SetupDiGetINFClassW.argtypes = (wintypes.LPCWSTR, ctypes.POINTER(GUID), wintypes.LPWSTR, wintypes.DWORD, ctypes.POINTER(wintypes.DWORD))

setupapi.SetupDiCreateDeviceInfoList.restype = wintypes.HANDLE
setupapi.SetupDiCreateDeviceInfoList.argtypes = (ctypes.POINTER(GUID), wintypes.HWND)

setupapi.SetupDiCreateDeviceInfoW.restype = wintypes.BOOL
setupapi.SetupDiCreateDeviceInfoW.argtypes = (wintypes.HANDLE, wintypes.LPCWSTR, ctypes.POINTER(GUID), wintypes.LPCWSTR, wintypes.HWND, wintypes.DWORD, ctypes.POINTER(SP_DEVINFO_DATA))

setupapi.SetupDiSetDeviceRegistryPropertyW.restype = wintypes.BOOL
setupapi.SetupDiSetDeviceRegistryPropertyW.argtypes = (wintypes.HANDLE, ctypes.POINTER(SP_DEVINFO_DATA), wintypes.DWORD, ctypes.POINTER(wintypes.BYTE), wintypes.DWORD)

setupapi.SetupDiCallClassInstaller.restype = wintypes.BOOL
setupapi.SetupDiCallClassInstaller.argtypes = (wintypes.DWORD, wintypes.HANDLE, ctypes.POINTER(SP_DEVINFO_DATA))

newdev.UpdateDriverForPlugAndPlayDevicesW.restype = wintypes.BOOL
newdev.UpdateDriverForPlugAndPlayDevicesW.argtypes = (wintypes.HWND, wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.DWORD, ctypes.POINTER(wintypes.BOOL))

setupapi.SetupDiDestroyDeviceInfoList.restype = wintypes.BOOL
setupapi.SetupDiDestroyDeviceInfoList.argtypes = (wintypes.HANDLE,)

setupapi.SetupDiGetClassDevsW.restype = wintypes.HANDLE
setupapi.SetupDiGetClassDevsW.argtypes = (ctypes.POINTER(GUID), wintypes.LPCWSTR, wintypes.HWND, wintypes.DWORD)

setupapi.SetupDiEnumDeviceInfo.restype = wintypes.BOOL
setupapi.SetupDiEnumDeviceInfo.argtypes = (wintypes.HANDLE, wintypes.DWORD, ctypes.POINTER(SP_DEVINFO_DATA))

setupapi.SetupDiGetDeviceRegistryPropertyW.restype = wintypes.BOOL
setupapi.SetupDiGetDeviceRegistryPropertyW.argtypes = (wintypes.HANDLE, ctypes.POINTER(SP_DEVINFO_DATA), wintypes.DWORD, ctypes.POINTER(wintypes.DWORD), wintypes.PBYTE, wintypes.DWORD, ctypes.POINTER(wintypes.DWORD))

def get_last_error():
    error_code = kernel32.GetLastError()
    return f"[WinError {error_code}] " + ctypes.FormatError(error_code).strip()

def ensure_admin():
    try:
        is_admin = (os.getuid() == 0)
    except AttributeError:
        is_admin = ctypes.windll.shell32.IsUserAnAdmin() != 0
    if not is_admin:
        print("[!] 检测到非管理员权限，正在尝试提权...")
        ctypes.windll.shell32.ShellExecuteW(None, "runas", sys.executable, " ".join(sys.argv), None, 1)
        sys.exit(0)

def check_device_exists():
    """检查具有指定HARDWARE_ID的设备是否已存在"""
    flags = DIGCF_PRESENT | DIGCF_ALLCLASSES
    dev_info_set = setupapi.SetupDiGetClassDevsW(None, None, None, flags)
    if dev_info_set == INVALID_HANDLE_VALUE:
        return False

    dev_index = 0
    while True:
        dev_info_data = SP_DEVINFO_DATA()
        dev_info_data.cbSize = ctypes.sizeof(SP_DEVINFO_DATA)
        if not setupapi.SetupDiEnumDeviceInfo(dev_info_set, dev_index, ctypes.byref(dev_info_data)):
            break
        
        dev_index += 1
        
        # 获取所需的缓冲区大小
        required_size = wintypes.DWORD()
        setupapi.SetupDiGetDeviceRegistryPropertyW(dev_info_set, ctypes.byref(dev_info_data), SPDRP_HARDWAREID, None, None, 0, ctypes.byref(required_size))
        
        if kernel32.GetLastError() != ERROR_INSUFFICIENT_BUFFER:
            continue

        # 分配缓冲区并获取Hardware ID
        hwid_buffer = ctypes.create_unicode_buffer(required_size.value)
        if setupapi.SetupDiGetDeviceRegistryPropertyW(dev_info_set, ctypes.byref(dev_info_data), SPDRP_HARDWAREID, None, ctypes.cast(hwid_buffer, wintypes.PBYTE), required_size.value, None):
            # Windows返回的是一个多字符串列表，我们只关心第一个
            if hwid_buffer.value.lower() == HARDWARE_ID.lower():
                setupapi.SetupDiDestroyDeviceInfoList(dev_info_set)
                return True

    setupapi.SetupDiDestroyDeviceInfoList(dev_info_set)
    return False

def install_driver():
    """安装虚拟设备和驱动"""
    if check_device_exists():
        print(f"[INFO] 设备 '{HARDWARE_ID}' 已存在，无需重复安装。")
        return

    inf_path = os.path.abspath(INF_FILENAME)
    if not os.path.exists(inf_path):
        raise Exception(f"INF文件 '{INF_FILENAME}' 在当前目录未找到。")

    print(f"[*] 准备安装设备, Hardware ID: {HARDWARE_ID}")
    print(f"[*] 使用INF文件: {inf_path}")

    # 步骤 1: 从INF文件获取ClassGuid
    print("[*] 步骤 1/5: 正在从INF文件获取设备类GUID...")
    class_guid = GUID()
    class_name = ctypes.create_unicode_buffer(MAX_CLASS_NAME_LEN)
    if not setupapi.SetupDiGetINFClassW(inf_path, ctypes.byref(class_guid), class_name, MAX_CLASS_NAME_LEN, None):
        raise Exception(f"获取INF设备类失败: {get_last_error()}")
    print(f"[*] 设备类: {class_name.value}")

    # 步骤 2: 创建设备信息列表
    print("[*] 步骤 2/5: 正在创建设备信息列表...")
    dev_info_set = setupapi.SetupDiCreateDeviceInfoList(ctypes.byref(class_guid), None)
    if dev_info_set == INVALID_HANDLE_VALUE:
        raise Exception(f"创建设备信息列表失败: {get_last_error()}")

    try:
        # 步骤 3: 创建设备信息元素
        dev_info_data = SP_DEVINFO_DATA()
        dev_info_data.cbSize = ctypes.sizeof(SP_DEVINFO_DATA)
        print(f"[*] 步骤 3/5: 正在创建设备实例 (Enumerator: {DEVICE_NAME})...")
        if not setupapi.SetupDiCreateDeviceInfoW(dev_info_set, DEVICE_NAME, ctypes.byref(class_guid), None, None, DICD_GENERATE_ID, ctypes.byref(dev_info_data)):
            raise Exception(f"创建设备信息失败: {get_last_error()}")

        # 步骤 4: 为新创建的设备设置Hardware ID
        print(f"[*] 步骤 4/5: 正在设置Hardware ID: {HARDWARE_ID}...")
        hwid_buffer = ctypes.create_unicode_buffer(HARDWARE_ID + '\0\0')
        if not setupapi.SetupDiSetDeviceRegistryPropertyW(dev_info_set, ctypes.byref(dev_info_data), SPDRP_HARDWAREID, ctypes.cast(hwid_buffer, ctypes.POINTER(wintypes.BYTE)), ctypes.sizeof(hwid_buffer)):
            raise Exception(f"设置Hardware ID失败: {get_last_error()}")

        # 步骤 5: 注册设备实例
        print("[*] 步骤 5/5: 正在注册设备实例...")
        if not setupapi.SetupDiCallClassInstaller(DIF_REGISTERDEVICE, dev_info_set, ctypes.byref(dev_info_data)):
            raise Exception(f"注册设备失败: {get_last_error()}")

        print("[*] 正在为新设备更新驱动...")
        needs_reboot = wintypes.BOOL()
        if not newdev.UpdateDriverForPlugAndPlayDevicesW(None, HARDWARE_ID, inf_path, DIIRFLAG_FORCE_INF, ctypes.byref(needs_reboot)):
            raise Exception(f"安装/更新驱动失败 (UpdateDriverForPlugAndPlayDevicesW): {get_last_error()}")
        
        if needs_reboot.value:
            print("[*] 系统提示需要重启才能完成安装。")
        
        print("\n[SUCCESS] 驱动和设备安装成功！")

    finally:
        print("[*] 清理设备信息列表...")
        setupapi.SetupDiDestroyDeviceInfoList(dev_info_set)

def uninstall_driver():
    """卸载虚拟设备和驱动"""
    print(f"[*] 正在查找设备 '{HARDWARE_ID}'...")
    
    flags = DIGCF_PRESENT | DIGCF_ALLCLASSES
    dev_info_set = setupapi.SetupDiGetClassDevsW(None, None, None, flags)
    if dev_info_set == INVALID_HANDLE_VALUE:
        raise Exception(f"获取设备列表失败: {get_last_error()}")

    device_found = False
    dev_index = 0
    try:
        while True:
            dev_info_data = SP_DEVINFO_DATA()
            dev_info_data.cbSize = ctypes.sizeof(SP_DEVINFO_DATA)
            if not setupapi.SetupDiEnumDeviceInfo(dev_info_set, dev_index, ctypes.byref(dev_info_data)):
                break
            
            dev_index += 1
            
            required_size = wintypes.DWORD()
            setupapi.SetupDiGetDeviceRegistryPropertyW(dev_info_set, ctypes.byref(dev_info_data), SPDRP_HARDWAREID, None, None, 0, ctypes.byref(required_size))
            
            if kernel32.GetLastError() != ERROR_INSUFFICIENT_BUFFER:
                continue

            hwid_buffer = ctypes.create_unicode_buffer(required_size.value)
            if setupapi.SetupDiGetDeviceRegistryPropertyW(dev_info_set, ctypes.byref(dev_info_data), SPDRP_HARDWAREID, None, ctypes.cast(hwid_buffer, wintypes.PBYTE), required_size.value, None):
                if hwid_buffer.value.lower() == HARDWARE_ID.lower():
                    device_found = True
                    print("[*] 找到设备，正在尝试卸载...")
                    if not setupapi.SetupDiCallClassInstaller(DIF_REMOVE, dev_info_set, ctypes.byref(dev_info_data)):
                        # 有时卸载后需要重启，可能会返回需要重启的错误，这里可以根据需要处理
                        error_code = get_last_error()
                        if "ERROR_PNP_REBOOT_REQUIRED" not in error_code:
                             raise Exception(f"卸载设备失败: {error_code}")
                        else:
                            print("[*] 设备已标记为卸载，需要重启系统来完成。")
                    else:
                        print("[*] 卸载API调用成功。")
                    break # 找到并处理后即可退出循环

    finally:
        setupapi.SetupDiDestroyDeviceInfoList(dev_info_set)

    if not device_found:
        print("[INFO] 未找到需要卸载的设备。")
    else:
        print("\n[SUCCESS] 卸载操作完成。")


if __name__ == "__main__":
    ensure_admin()
    if len(sys.argv) < 2 or sys.argv[1].lower() not in ['install', 'uninstall']:
        print("用法: python setup_driver.py [install | uninstall]")
        sys.exit(1)
    
    command = sys.argv[1].lower()
    
    try:
        if command == 'install':
            install_driver()
        elif command == 'uninstall':
            uninstall_driver()
    except Exception as e:
        print(f"\n[!] 操作失败: {e}")
    
    print("\n[*] 脚本执行完毕。")
    input("按 Enter 键退出...")