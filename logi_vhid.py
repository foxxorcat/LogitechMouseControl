import ctypes
import os
import time
from ctypes import wintypes, Structure, c_int8, c_uint8, c_int32, POINTER
from enum import IntEnum

class VHidResult(IntEnum):
    Success = 0
    Error = 1
    DeviceNotFound = 2
    AccessDenied = 3
    InvalidParameter = 4
    NotInitialized = 5

class MouseInput(Structure):
    _fields_ = [
        ('button', c_int8),   # 按钮状态
        ('x', c_int8),        # X轴移动 (-127 到 127)
        ('y', c_int8),        # Y轴移动 (-127 到 127)
        ('wheel', c_int8),    # 滚轮移动
        ('unk1', c_int8),     # 保留字段
    ]

class MouseButtons(IntEnum):
    NONE = 0
    LEFT = 1
    RIGHT = 2
    MIDDLE = 3
    
class KeyboardInput(Structure):
    _fields_ = [
        ('modifiers', c_uint8),           # 修饰键
        ('reserved', c_uint8),            # 保留字段
        ('keys', c_uint8 * 6),            # 按键码数组
    ]

class LogiVHid:
    """罗技虚拟HID设备Python封装类"""
    
    def __init__(self, dll_path=None):
        if dll_path is None:
            dll_path = self._find_dll()
        
        self._dll = ctypes.CDLL(dll_path)
        self._setup_prototypes()
        self._initialized = False
    
    def _find_dll(self):
        """自动查找DLL文件"""
        possible_paths = [
            "logi_vhid.dll",
            "target/release/logi_vhid.dll",
            "target/debug/logi_vhid.dll",
            "./logi_vhid.dll",
            "../target/release/logi_vhid.dll"
        ]
        
        for path in possible_paths:
            if os.path.exists(path):
                print(f"[+] 找到DLL: {path}")
                return path
        
        raise FileNotFoundError("未找到 logi_vhid.dll，请先编译动态库")
    
    def _setup_prototypes(self):
        """设置函数原型"""
        # 核心函数
        self._dll.vhid_initialize.restype = VHidResult
        self._dll.vhid_cleanup.restype = VHidResult
        self._dll.vhid_create_devices.restype = VHidResult
        self._dll.vhid_destroy_devices.restype = VHidResult
        
        # 输入函数
        self._dll.vhid_move_mouse.argtypes = [POINTER(MouseInput)]
        self._dll.vhid_move_mouse.restype = VHidResult
        
        self._dll.vhid_send_keyboard.argtypes = [POINTER(KeyboardInput)]
        self._dll.vhid_send_keyboard.restype = VHidResult
        
        # 便捷函数
        self._dll.vhid_mouse_move.argtypes = [c_int8, c_int8]
        self._dll.vhid_mouse_move.restype = VHidResult
        
        self._dll.vhid_mouse_click.argtypes = [c_int8]
        self._dll.vhid_mouse_click.restype = VHidResult
        
        self._dll.vhid_mouse_wheel.argtypes = [c_int8]
        self._dll.vhid_mouse_wheel.restype = VHidResult
        
        self._dll.vhid_mouse_down.argtypes = [ctypes.c_int8]
        self._dll.vhid_mouse_down.restype = VHidResult
        self._dll.vhid_mouse_up.restype = VHidResult
        
        self._dll.vhid_key_press.argtypes = [c_uint8]
        self._dll.vhid_key_press.restype = VHidResult
        
        self._dll.vhid_key_release.restype = VHidResult
        
        # 工具函数
        self._dll.vhid_get_last_error.argtypes = [wintypes.LPSTR, wintypes.DWORD]
        self._dll.vhid_get_last_error.restype = wintypes.DWORD
        
        self._dll.vhid_devices_created.restype = VHidResult
    
    def initialize(self):
        """初始化虚拟设备系统"""
        result = VHidResult(self._dll.vhid_initialize())
        if result == VHidResult.Success:
            self._initialized = True
        return result
    
    def cleanup(self):
        """清理虚拟设备系统"""
        result = VHidResult(self._dll.vhid_cleanup())
        if result == VHidResult.Success:
            self._initialized = False
        return result
    
    def create_devices(self):
        """创建虚拟HID设备"""
        if not self._initialized:
            return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_create_devices())
    
    def destroy_devices(self):
        """销毁虚拟HID设备"""
        if not self._initialized:
            return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_destroy_devices())
    
    def move_mouse(self, x=0, y=0):
        """移动鼠标相对坐标
        
        Args:
            x: X轴移动量 (-127 到 127)
            y: Y轴移动量 (-127 到 127)
        """
        if not self._initialized:
            return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_mouse_move(x, y))
    
    def mouse_click(self, button=1):
        """鼠标点击
        
        Args:
            button: 1-左键, 2-右键, 3-中键
        """
        if not self._initialized:
            return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_mouse_click(button))
    
    def mouse_wheel(self, delta=1):
        """鼠标滚轮
        
        Args:
            delta: 滚轮增量，正数向上，负数向下
        """
        if not self._initialized:
            return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_mouse_wheel(delta))
    
    def mouse_down(self, button: MouseButtons = MouseButtons.LEFT) -> VHidResult:
        return self._dll.vhid_mouse_down(button)
        
    def mouse_up(self) -> VHidResult:
        return self._dll.vhid_mouse_up()
    
    def key_press(self, key_code):
        """按下单个键盘按键
        
        Args:
            key_code: HID键码
        """
        if not self._initialized:
            return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_key_press(key_code))
    
    def key_release(self):
        """释放所有按键"""
        if not self._initialized:
            return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_key_release())
    
    def key_tap(self, key_code, delay=0.1):
        """按下并释放按键（组合操作）
        
        Args:
            key_code: HID键码
            delay: 按下和释放之间的延迟（秒）
        """
        result = self.key_press(key_code)
        if result != VHidResult.Success:
            return result
        
        time.sleep(delay)
        return self.key_release()
    
    def get_last_error(self):
        """获取最后错误信息"""
        buffer = ctypes.create_string_buffer(256)
        length = self._dll.vhid_get_last_error(buffer, 256)
        if length > 0:
            return buffer.value.decode('utf-8', errors='ignore')
        return "Unknown error"
    
    def devices_created(self):
        """检查设备是否已创建"""
        return self._dll.vhid_devices_created() != 0
    
    def is_initialized(self):
        """检查是否已初始化"""
        return self._initialized

# HID键码常量
class KeyCodes:
    A = 0x04
    B = 0x05
    C = 0x06
    D = 0x07
    E = 0x08
    F = 0x09
    G = 0x0A
    H = 0x0B
    I = 0x0C
    J = 0x0D
    K = 0x0E
    L = 0x0F
    M = 0x10
    N = 0x11
    O = 0x12
    P = 0x13
    Q = 0x14
    R = 0x15
    S = 0x16
    T = 0x17
    U = 0x18
    V = 0x19
    W = 0x1A
    X = 0x1B
    Y = 0x1C
    Z = 0x1D
    NUM_1 = 0x1E
    NUM_2 = 0x1F
    NUM_3 = 0x20
    NUM_4 = 0x21
    NUM_5 = 0x22
    NUM_6 = 0x23
    NUM_7 = 0x24
    NUM_8 = 0x25
    NUM_9 = 0x26
    NUM_0 = 0x27
    ENTER = 0x28
    ESC = 0x29
    BACKSPACE = 0x2A
    TAB = 0x2B
    SPACE = 0x2C
    MINUS = 0x2D
    EQUALS = 0x2E
    LEFT_BRACKET = 0x2F
    RIGHT_BRACKET = 0x30
    BACKSLASH = 0x31
    SEMICOLON = 0x33
    QUOTE = 0x34
    GRAVE = 0x35
    COMMA = 0x36
    PERIOD = 0x37
    SLASH = 0x38
    CAPS_LOCK = 0x39
    F1 = 0x3A
    F2 = 0x3B
    F3 = 0x3C
    F4 = 0x3D
    F5 = 0x3E
    F6 = 0x3F
    F7 = 0x40
    F8 = 0x41
    F9 = 0x42
    F10 = 0x43
    F11 = 0x44
    F12 = 0x45

class MouseButtons:
    LEFT = 1
    RIGHT = 2
    MIDDLE = 3