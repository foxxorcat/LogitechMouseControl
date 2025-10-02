# -*- coding: utf-8 -*-

import ctypes
import os
import sys
import json
from ctypes import wintypes

# ==============================================================================
# --- 1. 全局常量与定义 (综合自所有工作脚本)
# ==============================================================================

# --- 文件、设备与服务名称定义 ---
INF_BUS_FILE = "logi_joy_bus_enum.inf"
INF_HID_FILE = "logi_joy_vir_hid.inf"
HARDWARE_ID = "root\\LGHUBVirtualBus"
DEVICE_NAME = "System"  # PnP枚举器名称
BUS_DEVICE_PATH = "\\\\?\\root#system#0001#{dfbedcdb-2148-416d-9e4d-cecc2424128c}"
TEMP_ID_FILE = "temp_device_ids.json" # 用于在 create 和 destroy 命令间传递设备ID

# --- Windows API 常量 ---
INVALID_HANDLE_VALUE = ctypes.c_void_p(-1).value
GENERIC_READ = 0x80000000
GENERIC_WRITE = 0x40000000
OPEN_EXISTING = 3
SC_MANAGER_ALL_ACCESS = 0x000F003F
SERVICE_ALL_ACCESS = 0x000F01FF
ERROR_SERVICE_ALREADY_RUNNING = 1056
ERROR_INSUFFICIENT_BUFFER = 122

# SetupAPI 相关常量
DIF_REGISTERDEVICE = 0x00000019
DIF_REMOVE = 0x00000005
DIIRFLAG_FORCE_INF = 0x00000002
SPDRP_HARDWAREID = 0x00000001
DICD_GENERATE_ID = 0x00000001
DIGCF_ALLCLASSES = 0x00000004
MAX_CLASS_NAME_LEN = 32

# IOCTL 控制码 (DeviceIoControl Codes)
IOCTL_BUS_CREATE_DEVICE = 0x2A2000      # [已用] 发送到总线驱动，请求创建一个虚拟HID设备
IOCTL_BUS_DESTROY_DEVICE = 0x2A2004     # [已用] 发送到总线驱动，请求销毁一个虚拟HID设备

# --- 以下IOCTL常量在本脚本的简化版中未使用，但予以保留以供分析 ---
# IOCTL_BUS_PLUGIN_DEVICE = 0x2A2008      # 曾用于发送到总线驱动，模拟设备热插拔“连接”事件
# IOCTL_BUS_UNPLUG_DEVICE = 0x2A200C      # 曾用于发送到总线驱动，模拟设备热插拔“拔出”事件
# IOCTL_BUS_GET_DEVICE_INFO = 0x2A2010    # 曾用于发送到总线驱动，获取指定虚拟设备的状态信息
# IOCTL_XLCOR_GET_VERSION = 0x2A2014      # 曾用于发送到虚拟HID设备，获取其功能驱动的版本号
# IOCTL_XLCOR_GET_CONFIG = 0x2A201C       # 曾用于发送到虚拟HID设备，从注册表读取驱动配置
# IOCTL_XLCOR_SET_CONFIG = 0x2A2020       # 曾用于发送到虚拟HID设备，向注册表写入驱动配置 (高危)

# --- 加载 Windows API 动态链接库 ---
kernel32 = ctypes.WinDLL('kernel32', use_last_error=True)
advapi32 = ctypes.WinDLL('advapi32', use_last_error=True)
shell32 = ctypes.WinDLL('shell32', use_last_error=True)
setupapi = ctypes.WinDLL('setupapi', use_last_error=True)
newdev = ctypes.WinDLL('newdev', use_last_error=True)


# ==============================================================================
# --- 2. ctypes 结构体与函数原型 (采用最稳定、最完整的定义)
# ==============================================================================

class GUID(ctypes.Structure):
    """表示一个全局唯一标识符 (GUID)"""
    _fields_ = [
        ("Data1", wintypes.DWORD), ("Data2", wintypes.WORD),
        ("Data3", wintypes.WORD), ("Data4", (ctypes.c_ubyte * 8))
    ]

class SP_DEVINFO_DATA(ctypes.Structure):
    """表示一个设备实例(Device Instance)。它不包含设备的详细信息，而是一个引用或“句柄”，SetupAPI通过它来对特定设备执行操作。"""
    _fields_ = [
        ("cbSize", wintypes.DWORD), ("ClassGuid", GUID),
        ("DevInst", wintypes.DWORD), ("Reserved", ctypes.POINTER(wintypes.ULONG))
    ]

wintypes.SC_HANDLE = wintypes.HANDLE
LPCWSTR, PBYTE = wintypes.LPCWSTR, wintypes.PBYTE
shell32.IsUserAnAdmin.restype = wintypes.BOOL

# --- 关键：为所有API函数预先定义参数类型(argtypes)和返回类型(restype)。---
# 这样做可以确保Python数据类型(如 int, str)能被正确地转换为C语言需要的类型(如 DWORD, LPCWSTR)，
# 尤其能避免在处理指针和句柄时出现64位/32位不兼容的错误，是ctypes稳定运行的保证。
setupapi.SetupDiGetINFClassW.restype = wintypes.BOOL
setupapi.SetupDiGetINFClassW.argtypes = (wintypes.LPCWSTR, ctypes.POINTER(GUID), wintypes.LPWSTR, wintypes.DWORD, ctypes.POINTER(wintypes.DWORD))
setupapi.SetupDiCreateDeviceInfoList.restype = wintypes.HANDLE
setupapi.SetupDiCreateDeviceInfoList.argtypes = (ctypes.POINTER(GUID), wintypes.HWND)
setupapi.SetupDiCreateDeviceInfoW.restype = wintypes.BOOL
setupapi.SetupDiCreateDeviceInfoW.argtypes = (wintypes.HANDLE, wintypes.LPCWSTR, ctypes.POINTER(GUID), wintypes.LPCWSTR, wintypes.HWND, wintypes.DWORD, ctypes.POINTER(SP_DEVINFO_DATA))
setupapi.SetupDiSetDeviceRegistryPropertyW.restype = wintypes.BOOL
setupapi.SetupDiSetDeviceRegistryPropertyW.argtypes = (wintypes.HANDLE, ctypes.POINTER(SP_DEVINFO_DATA), wintypes.DWORD, PBYTE, wintypes.DWORD)
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
setupapi.SetupDiGetDeviceRegistryPropertyW.argtypes = (wintypes.HANDLE, ctypes.POINTER(SP_DEVINFO_DATA), wintypes.DWORD, ctypes.POINTER(wintypes.DWORD), PBYTE, wintypes.DWORD, ctypes.POINTER(wintypes.DWORD))
setupapi.SetupCopyOEMInfW.restype = wintypes.BOOL
setupapi.SetupCopyOEMInfW.argtypes = (wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.UINT, wintypes.UINT, wintypes.LPWSTR, wintypes.UINT, ctypes.POINTER(wintypes.UINT), ctypes.POINTER(wintypes.LPWSTR))


# ==============================================================================
# --- 3. 辅助及通用功能函数
# ==============================================================================

def get_last_error():
    """获取并格式化最后一次Windows API调用产生的错误信息"""
    error_code = kernel32.GetLastError()
    return f"[WinError {error_code}] " + ctypes.FormatError(error_code).strip()

def ensure_admin():
    """检查管理员权限，如果不足则尝试提权"""
    if not shell32.IsUserAnAdmin():
        print("[!] 权限不足, 正在尝试以管理员身份重新运行...")
        ret = shell32.ShellExecuteW(None, "runas", sys.executable, " ".join(sys.argv), None, 1)
        if ret <= 32:
            print(f"[!] 提权失败，错误码: {ret}。请右键点击脚本并选择'以管理员身份运行'。")
        sys.exit(0)

def load_device_ids():
    """从临时JSON文件中加载已创建的设备ID"""
    if not os.path.exists(TEMP_ID_FILE):
        raise FileNotFoundError(f"找不到设备ID文件 '{TEMP_ID_FILE}'。请先执行 'create-hid' 命令。")
    with open(TEMP_ID_FILE, 'r') as f:
        return json.load(f)

# ==============================================================================
# --- 4. 驱动与设备管理 (install, uninstall) - 采用已知工作脚本的逻辑
# ==============================================================================

def install_driver():
    """
    安装驱动程序。此函数严格遵循可靠的SetupAPI调用流程。
    首先创建并安装核心的虚拟总线设备，然后安装其他依赖的驱动（如HID驱动）。
    """
    print("[*] --- 开始驱动安装流程 ---")

    # --- 步骤1: 创建和安装核心总线设备 ---
    inf_bus_path = os.path.abspath(INF_BUS_FILE)
    if not os.path.exists(inf_bus_path):
        raise FileNotFoundError(f"核心总线驱动 '{INF_BUS_FILE}' 未找到!")

    if check_device_exists():
        print(f"[*] 设备 '{HARDWARE_ID}' 已存在，跳过创建。")
    else:
        print(f"[*] 开始创建并安装设备: {HARDWARE_ID}")
        
        # 流程 1.1: 从INF文件中解析设备的类GUID。GUID是Windows用来唯一标识对象（如设备类）的128位数字。
        # 这一步是为了告诉系统我们想创建一个“什么类型”的设备。
        print("[*] 步骤 1/6: 从INF文件获取设备类GUID...")
        class_guid = GUID()
        class_name = ctypes.create_unicode_buffer(MAX_CLASS_NAME_LEN)
        if not setupapi.SetupDiGetINFClassW(inf_bus_path, ctypes.byref(class_guid), class_name, MAX_CLASS_NAME_LEN, None):
            raise Exception(f"获取INF设备类失败: {get_last_error()}")
        print(f"  - 成功获取设备类: {class_name.value}")

        # 流程 1.2: 创建一个设备信息集(Device Info Set)，它是一个用于处理一组设备信息的容器。
        print("[*] 步骤 2/6: 正在创建设备信息列表...")
        dev_info_set = setupapi.SetupDiCreateDeviceInfoList(ctypes.byref(class_guid), None)
        if dev_info_set == INVALID_HANDLE_VALUE:
            raise Exception(f"创建设备信息列表失败: {get_last_error()}")

        try:
            # 流程 1.3: 在设备信息集中创建一个具体的设备实例(Device Info Element)。此时它还只是一个内存中的占位符，并未在系统中实际注册。
            # [关键] 必须使用两步初始化，先创建实例再设置cbSize，确保结构体被正确填充。
            dev_info_data = SP_DEVINFO_DATA()
            dev_info_data.cbSize = ctypes.sizeof(SP_DEVINFO_DATA)

            print(f"[*] 步骤 3/6: 正在创建设备实例...")
            if not setupapi.SetupDiCreateDeviceInfoW(dev_info_set, DEVICE_NAME, ctypes.byref(class_guid), None, None, DICD_GENERATE_ID, ctypes.byref(dev_info_data)):
                raise Exception(f"创建设备信息失败: {get_last_error()}")

            # 流程 1.4: 为这个内存中的设备实例设置硬件ID(Hardware ID)，这是设备即插即用(PnP)的核心标识符。
            print(f"[*] 步骤 4/6: 正在设置Hardware ID: {HARDWARE_ID}...")
            hwid_buffer = ctypes.create_unicode_buffer(HARDWARE_ID + '\0\0')
            if not setupapi.SetupDiSetDeviceRegistryPropertyW(dev_info_set, ctypes.byref(dev_info_data), SPDRP_HARDWAREID, ctypes.cast(hwid_buffer, PBYTE), ctypes.sizeof(hwid_buffer)):
                raise Exception(f"设置Hardware ID失败: {get_last_error()}")

            # 流程 1.5: 调用类安装程序(Class Installer)来正式在系统中注册这个设备实例。
            # 这一步完成后，设备就会出现在设备管理器中（可能带黄色感叹号，因为驱动还未安装）。
            print("[*] 步骤 5/6: 正在注册设备实例...")
            if not setupapi.SetupDiCallClassInstaller(DIF_REGISTERDEVICE, dev_info_set, ctypes.byref(dev_info_data)):
                raise Exception(f"注册设备失败: {get_last_error()}")
            
            # 流程 1.6: 为这个新注册的设备安装或更新驱动程序。
            # UpdateDriverForPlugAndPlayDevices会根据提供的硬件ID搜索匹配的INF文件并执行安装。
            print("[*] 步骤 6/6: 正在为新设备更新驱动...")
            needs_reboot = wintypes.BOOL()
            if not newdev.UpdateDriverForPlugAndPlayDevicesW(None, HARDWARE_ID, inf_bus_path, DIIRFLAG_FORCE_INF, ctypes.byref(needs_reboot)):
                raise Exception(f"安装/更新驱动失败: {get_last_error()}")
            
            print("[+] 核心总线设备创建并安装成功！")
            if needs_reboot.value:
                print("[注意] 系统提示需要重启。")
        finally:
            # 无论成功失败，都必须销毁设备信息集句柄，防止资源泄露
            setupapi.SetupDiDestroyDeviceInfoList(dev_info_set)
            
    # --- 步骤2: 安装其他相关驱动 (例如虚拟HID驱动) ---
    inf_hid_path = os.path.abspath(INF_HID_FILE)
    if not os.path.exists(inf_hid_path):
        print(f"[警告] HID驱动 '{INF_HID_FILE}' 未找到，虚拟设备可能无法工作。")
    else:
        print(f"[*] 正在安装HID驱动 '{INF_HID_FILE}'...")
        # SetupCopyOEMInf 是一个标准、可靠的驱动安装方法，它会将驱动复制到系统驱动仓库(%SystemRoot%\inf)中，以便系统后续可以找到它。
        if setupapi.SetupCopyOEMInfW(inf_hid_path, None, 1, 0, None, 0, None, None):
            print(f"  - [成功] 驱动 '{INF_HID_FILE}' 安装/更新成功。")
        else:
            error = get_last_error()
            if "ERROR_FILE_EXISTS" in error:
                print(f"  - [信息] 驱动 '{INF_HID_FILE}' 已存在于驱动仓库中。")
            else:
                raise ctypes.WinError(ctypes.get_last_error())

    print("\n[*] 驱动安装流程执行完毕！")


def uninstall_driver():
    """
    卸载虚拟设备和驱动。
    """
    print(f"[*] 开始卸载流程，目标设备: '{HARDWARE_ID}'...")
    
    # 使用 DIGCF_ALLCLASSES 标志来搜索所有已为系统配置的设备，无论其当前是否在线
    flags = DIGCF_ALLCLASSES
    dev_info_set = setupapi.SetupDiGetClassDevsW(None, None, None, flags)
    if dev_info_set == INVALID_HANDLE_VALUE:
        raise Exception(f"获取设备列表失败: {get_last_error()}")

    device_found = False
    try:
        dev_index = 0
        while True:
            # 遍历系统中的每一个设备
            dev_info_data = SP_DEVINFO_DATA()
            dev_info_data.cbSize = ctypes.sizeof(SP_DEVINFO_DATA)
            if not setupapi.SetupDiEnumDeviceInfo(dev_info_set, dev_index, ctypes.byref(dev_info_data)):
                break # 遍历完毕
            dev_index += 1
            
            # 获取设备的硬件ID
            required_size = wintypes.DWORD()
            # 第一次调用获取需要的缓冲区大小
            setupapi.SetupDiGetDeviceRegistryPropertyW(dev_info_set, ctypes.byref(dev_info_data), SPDRP_HARDWAREID, None, None, 0, ctypes.byref(required_size))
            if kernel32.GetLastError() != ERROR_INSUFFICIENT_BUFFER:
                continue

            # 第二次调用获取实际数据
            hwid_buffer = ctypes.create_unicode_buffer(required_size.value)
            if setupapi.SetupDiGetDeviceRegistryPropertyW(dev_info_set, ctypes.byref(dev_info_data), SPDRP_HARDWAREID, None, ctypes.cast(hwid_buffer, PBYTE), required_size.value, None):
                # 检查硬件ID是否与我们的目标匹配
                if hwid_buffer.value.lower() == HARDWARE_ID.lower():
                    device_found = True
                    print("[*] 找到匹配的设备，正在执行卸载...")
                    # 调用标准的设备移除安装程序
                    if not setupapi.SetupDiCallClassInstaller(DIF_REMOVE, dev_info_set, ctypes.byref(dev_info_data)):
                        error = get_last_error()
                        if "ERROR_PNP_REBOOT_REQUIRED" not in error:
                            raise Exception(f"卸载设备失败: {error}")
                        else:
                            print("[*] 设备已标记为卸载，需要重启系统来完成。")
                    else:
                        print("[+] 卸载API调用成功。")
                    break # 找到并处理后即可退出循环
    finally:
        setupapi.SetupDiDestroyDeviceInfoList(dev_info_set)

    if not device_found:
        print("[信息] 未找到需要卸载的设备。")
    else:
        print("\n[成功] 卸载操作完成。")

def check_device_exists():
    """检查具有指定HARDWARE_ID的设备是否已存在"""
    flags = DIGCF_ALLCLASSES
    dev_info_set = setupapi.SetupDiGetClassDevsW(None, None, None, flags)
    if dev_info_set == INVALID_HANDLE_VALUE: return False
    
    try:
        dev_index = 0
        while True:
            dev_info_data = SP_DEVINFO_DATA()
            dev_info_data.cbSize = ctypes.sizeof(SP_DEVINFO_DATA)
            if not setupapi.SetupDiEnumDeviceInfo(dev_info_set, dev_index, ctypes.byref(dev_info_data)): break
            dev_index += 1
            
            required_size = wintypes.DWORD()
            setupapi.SetupDiGetDeviceRegistryPropertyW(dev_info_set, ctypes.byref(dev_info_data), SPDRP_HARDWAREID, None, None, 0, ctypes.byref(required_size))
            if kernel32.GetLastError() != ERROR_INSUFFICIENT_BUFFER: continue
            
            hwid_buffer = ctypes.create_unicode_buffer(required_size.value)
            if setupapi.SetupDiGetDeviceRegistryPropertyW(dev_info_set, ctypes.byref(dev_info_data), SPDRP_HARDWAREID, None, ctypes.cast(hwid_buffer, PBYTE), required_size.value, None):
                if hwid_buffer.value.lower() == HARDWARE_ID.lower():
                    return True
    finally:
        setupapi.SetupDiDestroyDeviceInfoList(dev_info_set)
    return False

# ==============================================================================
# --- 5. 虚拟HID设备管理 (create, destroy) - 采用已知工作脚本的逻辑
# ==============================================================================
def create_hid_devices():
    """
    通过与总线驱动进行IOCTL通信，请求创建虚拟的键盘和鼠标设备。
    这模拟了物理设备插入总线的行为。
    """
    print("[*] --- 开始创建虚拟HID设备 ---")

    print(f"[*] 正在打开总线设备句柄: {BUS_DEVICE_PATH}...")
    bus_handle = kernel32.CreateFileW(BUS_DEVICE_PATH, GENERIC_READ | GENERIC_WRITE, 0, None, OPEN_EXISTING, 0, None)
    if bus_handle == INVALID_HANDLE_VALUE:
        raise ctypes.WinError(ctypes.get_last_error())
    print("  - 总线设备句柄获取成功。")

    created_ids = {}
    try:
        # 尝试创建虚拟键盘
        try:
            keyboard_id = _create_single_hid_device(bus_handle, 'keyboard')
            created_ids['keyboard_id'] = keyboard_id
        except Exception as e:
            print(f"[!] 创建虚拟键盘时出错: {e}")

        # 尝试创建虚拟鼠标
        try:
            mouse_id = _create_single_hid_device(bus_handle, 'mouse')
            created_ids['mouse_id'] = mouse_id
        except Exception as e:
            print(f"[!] 创建虚拟鼠标时出错: {e}")

        if not created_ids:
            print("[!] 未能成功创建任何虚拟HID设备。")
        else:
            # 将成功创建的设备ID保存到文件，供destroy命令使用
            with open(TEMP_ID_FILE, 'w') as f:
                json.dump(created_ids, f)
            print(f"\n[成功] 虚拟HID设备创建完毕，ID已保存至 '{TEMP_ID_FILE}'。")
    finally:
        kernel32.CloseHandle(bus_handle)

def destroy_hid_devices():
    """
    通过IOCTL通信，请求销毁之前创建的虚拟键盘和鼠标设备，并清理临时文件。
    """
    print("[*] --- 开始销毁虚拟HID设备 ---")
    device_ids = load_device_ids()
    
    keyboard_id = device_ids.get('keyboard_id')
    mouse_id = device_ids.get('mouse_id')

    print(f"[*] 正在打开总线设备句柄: {BUS_DEVICE_PATH}...")
    bus_handle = kernel32.CreateFileW(BUS_DEVICE_PATH, GENERIC_READ | GENERIC_WRITE, 0, None, OPEN_EXISTING, 0, None)
    if bus_handle == INVALID_HANDLE_VALUE:
        raise ctypes.WinError(ctypes.get_last_error())
    print("  - 总线设备句柄获取成功。")
    
    try:
        _destroy_single_hid_device(bus_handle, keyboard_id, 'keyboard')
        _destroy_single_hid_device(bus_handle, mouse_id, 'mouse')
    finally:
        kernel32.CloseHandle(bus_handle)
    
    if os.path.exists(TEMP_ID_FILE):
        os.remove(TEMP_ID_FILE)
        print(f"[*] 已删除临时ID文件 '{TEMP_ID_FILE}'。")
    print(f"\n[成功] 虚拟HID设备清理完毕。")

def _create_single_hid_device(bus_handle, device_type):
    """
    (内部函数) 构造并发送IOCTL来创建单个虚拟设备。
    此函数中的数据包结构是逆向分析得出的，必须精确匹配。
    """
    bytes_returned = wintypes.DWORD()
    if device_type == 'keyboard':
        buffer = ctypes.create_string_buffer(246)
        pid_vid_part = "LGHUBDevice\\VID_046D&PID_C232".encode('utf-16-le')
        magic_int1, dev_type_flag = -1036909459, 0
        hid_desc = bytes([0x05, 0x01, 0x09, 0x06, 0xA1, 0x01, 0x05, 0x07, 0x19, 0xE0, 0x29, 0xE7, 0x15, 0x00, 0x25, 0x01, 0x75, 0x01, 0x95, 0x08, 0x81, 0x02, 0x95, 0x01, 0x75, 0x08, 0x81, 0x01, 0x95, 0x05, 0x75, 0x01, 0x05, 0x08, 0x19, 0x01, 0x29, 0x05, 0x91, 0x02, 0x95, 0x01, 0x75, 0x03, 0x91, 0x01, 0x95, 0x06, 0x75, 0x08, 0x15, 0x00, 0x25, 0x65, 0x05, 0x07, 0x19, 0x00, 0x29, 0x65, 0x81, 0x00, 0xC0])
    elif device_type == 'mouse':
        buffer = ctypes.create_string_buffer(254)
        pid_vid_part = "LGHUBDevice\\VID_046D&PID_C231".encode('utf-16-le')
        magic_int1, dev_type_flag = -1036974995, 1
        hid_desc = bytes([0x05, 0x01, 0x09, 0x02, 0xA1, 0x01, 0x09, 0x01, 0xA1, 0x00, 0x05, 0x09, 0x19, 0x01, 0x29, 0x05, 0x15, 0x00, 0x25, 0x01, 0x95, 0x05, 0x75, 0x01, 0x81, 0x02, 0x95, 0x01, 0x75, 0x03, 0x81, 0x01, 0x05, 0x01, 0x09, 0x30, 0x09, 0x31, 0x09, 0x38, 0x15, 0x81, 0x25, 0x7F, 0x75, 0x08, 0x95, 0x03, 0x81, 0x06, 0xC0, 0xC0])
    else:
        raise ValueError("无效的设备类型")
    
    print(f"[+] 正在构建并发送“创建{device_type}”的IOCTL数据包...")
    ctypes.memset(buffer, 0, ctypes.sizeof(buffer))
    # 填充IOCTL输入缓冲区的各个字段
    ctypes.memmove(ctypes.addressof(buffer) + 0,   ctypes.byref(ctypes.c_uint32(183)), 4)
    ctypes.memmove(ctypes.addressof(buffer) + 8,   ctypes.byref(ctypes.c_uint32(1)), 4)
    ctypes.memmove(ctypes.addressof(buffer) + 12,  ctypes.byref(ctypes.c_uint32(62)), 4)
    ctypes.memmove(ctypes.addressof(buffer) + 16,  pid_vid_part, len(pid_vid_part))
    ctypes.memmove(ctypes.addressof(buffer) + 144, ctypes.byref(ctypes.c_int32(magic_int1)), 4)
    ctypes.memmove(ctypes.addressof(buffer) + 148, ctypes.byref(ctypes.c_uint32(dev_type_flag)), 4)
    ctypes.memmove(ctypes.addressof(buffer) + 182, hid_desc, len(hid_desc))
    ctypes.memmove(ctypes.addressof(buffer) + 178, ctypes.byref(ctypes.c_uint32(len(hid_desc))), 4)

    # 发送设备IO控制请求
    if not kernel32.DeviceIoControl(bus_handle, IOCTL_BUS_CREATE_DEVICE, buffer, ctypes.sizeof(buffer), buffer, ctypes.sizeof(buffer), ctypes.byref(bytes_returned), None):
        raise ctypes.WinError(ctypes.get_last_error())
    
    # 从输出缓冲区中解析驱动返回的设备ID
    device_id = ctypes.c_uint32.from_address(ctypes.addressof(buffer) + 4).value
    print(f"  - IOCTL发送成功，获得 {device_type} 设备ID: {device_id}")
    return device_id

def _destroy_single_hid_device(bus_handle, device_id, device_type):
    """(内部函数) 构造并发送IOCTL来销毁单个虚拟设备"""
    if not device_id:
        print(f"[*] 未找到要销毁的 {device_type} 设备ID，跳过。")
        return
        
    print(f"[+] 正在发送“销毁{device_type}”的IOCTL数据包 (ID: {device_id})...")
    buffer = ctypes.create_string_buffer(20)
    ctypes.memset(buffer, 0, 20)
    type_flag = 0 if device_type == 'keyboard' else 1
    
    ctypes.memmove(ctypes.addressof(buffer) + 0, ctypes.byref(ctypes.c_uint32(20)), 4)
    ctypes.memmove(ctypes.addressof(buffer) + 4, ctypes.byref(ctypes.c_uint32(device_id)), 4)
    ctypes.memmove(ctypes.addressof(buffer) + 8, ctypes.byref(ctypes.c_uint32(type_flag)), 4)
    
    try:
        if not kernel32.DeviceIoControl(bus_handle, IOCTL_BUS_DESTROY_DEVICE, buffer, ctypes.sizeof(buffer), None, 0, ctypes.byref(wintypes.DWORD()), None):
            raise ctypes.WinError(ctypes.get_last_error())
        print(f"  - 成功发送销毁请求。")
    except Exception as e:
        print(f"[!] 销毁设备 {device_id} 时出错: {e}")

# ==============================================================================
# --- 6. 主程序入口
# ==============================================================================

if __name__ == "__main__":
    ensure_admin()
    
    commands = ['install', 'uninstall', 'create-hid', 'destroy-hid']
    if len(sys.argv) != 2 or sys.argv[1].lower() not in commands:
        print("\n用法: python your_script_name.py [命令]")
        print("\n核心命令:")
        print("  install      - 安装罗技虚拟总线驱动和设备。")
        print("  uninstall    - 卸载罗技虚拟总线驱动和设备。")
        print("  create-hid   - 创建虚拟键盘和鼠标设备。")
        print("  destroy-hid  - 销毁已创建的虚拟键盘和鼠标设备。")
        sys.exit(1)

    command = sys.argv[1].lower()
    
    try:
        if command == 'install':
            install_driver()
        elif command == 'uninstall':
            uninstall_driver()
        elif command == 'create-hid':
            create_hid_devices()
        elif command == 'destroy-hid':
            destroy_hid_devices()
            
    except Exception as e:
        print(f"\n[错误] 执行 '{command}' 命令时发生严重错误: {e}")
        import traceback
        traceback.print_exc() # 打印详细的错误堆栈，方便进一步调试
    
    print("\n[*] 脚本执行完毕。")
    input("按 Enter 键退出...")

