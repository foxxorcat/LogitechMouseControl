import ctypes
import time

# --- Windows API & 常量定义 ---

# 从 kernel32.dll 导入函数
kernel32 = ctypes.WinDLL('kernel32', use_last_error=True)

# 定义函数原型
CreateFileW = kernel32.CreateFileW
DeviceIoControl = kernel32.DeviceIoControl
CloseHandle = kernel32.CloseHandle

# 定义 CreateFileW 所需的常量
GENERIC_WRITE = 0x40000000
FILE_SHARE_READ = 0x00000001
FILE_SHARE_WRITE = 0x00000002
OPEN_EXISTING = 3

# 定义 DeviceIoControl 的 IOCTL 代码
IOCTL_VULNERABLE_CODE = 0x2a2010

# 定义设备路径
DEVICE_PATHS = [
    "\\\\.\\ROOT#SYSTEM#0002#{1abc05c0-c378-41b9-9cef-df1aba82b015}",
    "\\\\.\\ROOT#SYSTEM#0001#{1abc05c0-c378-41b9-9cef-df1aba82b015}",
]

# --- 模拟 C 代码中的 MOUSE_IO 结构体 ---
class MOUSE_IO(ctypes.Structure):
    _fields_ = [
        ("button", ctypes.c_char),
        ("x",      ctypes.c_char),
        ("y",      ctypes.c_char),
        ("wheel",  ctypes.c_char),
        ("unk1",   ctypes.c_char),
    ]

# --- 主逻辑 ---

def get_driver_handle():
    """尝试打开驱动程序并返回句柄"""
    for device_path in DEVICE_PATHS:
        print(f"[*] 正在尝试打开设备: {device_path}")
        handle = CreateFileW(
            ctypes.c_wchar_p(device_path),
            GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            0,
            None
        )
        
        # Windows API 中，无效句柄的值是 -1
        if handle != -1:
            print(f"[+] 成功获取驱动句柄: {handle}")
            return handle
    
    print("[-] 未能找到驱动设备或获取句柄失败。")
    return None

def move_mouse(handle, x, y, button=0, wheel=0):
    """向驱动发送鼠标移动命令"""
    if not handle:
        return False
        
    # 填充 MOUSE_IO 结构体
    mouse_input = MOUSE_IO(
        button=button.to_bytes(1, 'little', signed=True),
        x=x.to_bytes(1, 'little', signed=True),
        y=y.to_bytes(1, 'little', signed=True),
        wheel=wheel.to_bytes(1, 'little', signed=True),
        unk1=b'\x00'
    )
    
    bytes_returned = ctypes.c_ulong()
    
    # 调用 DeviceIoControl
    success = DeviceIoControl(
        handle,
        IOCTL_VULNERABLE_CODE,
        ctypes.byref(mouse_input), # 输入缓冲区
        ctypes.sizeof(mouse_input),  # 输入缓冲区大小
        None, # 输出缓冲区
        0,    # 输出缓冲区大小
        ctypes.byref(bytes_returned),
        None
    )
    
    return success != 0

if __name__ == "__main__":
    driver_handle = get_driver_handle()
    
    if driver_handle:
        print("\n[!] 驱动句柄已获得，将在2秒后开始移动鼠标...")
        time.sleep(2)
        
        # 循环移动鼠标30次
        for i in range(30):
            print(f"[+] 正在移动鼠标... (第 {i+1}/30 次)")
            # 向上移动10个单位 (Y轴为负)
            if not move_mouse(driver_handle, x=0, y=-10):
                print("[-] 发送移动命令失败。")
                break
            time.sleep(0.1) # 停顿0.1秒，让移动效果更明显
            
        print("\n[*] 测试完成。")
        # 关闭句柄
        CloseHandle(driver_handle)