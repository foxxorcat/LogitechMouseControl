import ctypes
import os
import time
from ctypes import wintypes, Structure, c_int8, c_uint8, c_int32, POINTER
from enum import IntEnum

class VHidResult(IntEnum):
    """Result status codes from the VHID library."""
    Success = 0
    Error = 1
    DeviceNotFound = 2
    AccessDenied = 3
    InvalidParameter = 4
    NotInitialized = 5

class MouseInput(Structure):
    """Represents a mouse input report."""
    _fields_ = [
        ('button', c_int8),   # Button status (bitmask)
        ('x', c_int8),        # X-axis movement (-127 to 127)
        ('y', c_int8),        # Y-axis movement (-127 to 127)
        ('wheel', c_int8),    # Wheel movement
        ('reserved', c_int8), # Reserved field
    ]

class KeyboardInput(Structure):
    """Represents a keyboard input report."""
    _fields_ = [
        ('modifiers', c_uint8),      # Modifier keys (bitmask)
        ('reserved', c_uint8),       # Reserved field
        ('keys', c_uint8 * 6),       # Array of pressed key codes
    ]

class MouseButtons(IntEnum):
    """Mouse button constants (bitmask)."""
    LEFT = 1
    RIGHT = 2
    MIDDLE = 4

class KeyModifiers(IntEnum):
    """Keyboard modifier constants (bitmask)."""
    LEFT_CTRL = 0x01
    LEFT_SHIFT = 0x02
    LEFT_ALT = 0x04
    LEFT_GUI = 0x08
    RIGHT_CTRL = 0x10
    RIGHT_SHIFT = 0x20
    RIGHT_ALT = 0x40
    RIGHT_GUI = 0x80

class LogiVHid:
    """Python wrapper for the Logitech Virtual HID device library."""
    
    def __init__(self, dll_path=None):
        if dll_path is None:
            dll_path = self._find_dll()
        
        self._dll = ctypes.CDLL(dll_path)
        self._setup_prototypes()
        self._initialized = False
    
    def _find_dll(self):
        """Automatically find the DLL file in common paths."""
        possible_paths = [
            "logi_vhid.dll",
            "target/release/logi_vhid.dll",
            "target/debug/logi_vhid.dll",
            "./logi_vhid.dll",
            "../target/release/logi_vhid.dll"
        ]
        
        for path in possible_paths:
            if os.path.exists(path):
                print(f"[+] Found DLL: {os.path.abspath(path)}")
                return os.path.abspath(path)
        
        raise FileNotFoundError("Could not find logi_vhid.dll. Please compile the Rust library first.")
    
    def _setup_prototypes(self):
        """Set up function prototypes for the loaded DLL."""
        # Core functions
        self._dll.vhid_initialize.restype = VHidResult
        self._dll.vhid_cleanup.restype = VHidResult
        self._dll.vhid_power_on.restype = VHidResult
        self._dll.vhid_power_off.restype = VHidResult
        self._dll.vhid_reset_state.restype = VHidResult

        # Raw report sending functions
        self._dll.vhid_send_mouse_report.argtypes = [POINTER(MouseInput)]
        self._dll.vhid_send_mouse_report.restype = VHidResult
        self._dll.vhid_send_keyboard_report.argtypes = [POINTER(KeyboardInput)]
        self._dll.vhid_send_keyboard_report.restype = VHidResult

        # High-level mouse functions
        self._dll.vhid_mouse_move_absolute.argtypes = [c_int32, c_int32]
        self._dll.vhid_mouse_move_absolute.restype = VHidResult
        self._dll.vhid_mouse_move.argtypes = [c_int8, c_int8]
        self._dll.vhid_mouse_move.restype = VHidResult
        self._dll.vhid_mouse_down.argtypes = [c_int8]
        self._dll.vhid_mouse_down.restype = VHidResult
        self._dll.vhid_mouse_up.argtypes = [c_int8]
        self._dll.vhid_mouse_up.restype = VHidResult
        self._dll.vhid_mouse_click.argtypes = [c_int8]
        self._dll.vhid_mouse_click.restype = VHidResult
        self._dll.vhid_mouse_wheel.argtypes = [c_int8]
        self._dll.vhid_mouse_wheel.restype = VHidResult

        # High-level keyboard functions
        self._dll.vhid_key_down.argtypes = [c_uint8]
        self._dll.vhid_key_down.restype = VHidResult
        self._dll.vhid_key_up.argtypes = [c_uint8]
        self._dll.vhid_key_up.restype = VHidResult
        self._dll.vhid_modifier_down.argtypes = [c_uint8]
        self._dll.vhid_modifier_down.restype = VHidResult
        self._dll.vhid_modifier_up.argtypes = [c_uint8]
        self._dll.vhid_modifier_up.restype = VHidResult
        self._dll.vhid_key_tap.argtypes = [c_uint8]
        self._dll.vhid_key_tap.restype = VHidResult

        # Utility functions
        self._dll.vhid_get_last_error.argtypes = [wintypes.LPSTR, ctypes.c_size_t]
        self._dll.vhid_get_last_error.restype = ctypes.c_size_t
    
    def initialize(self) -> VHidResult:
        """Initializes the virtual device system's manager."""
        result = VHidResult(self._dll.vhid_initialize())
        if result == VHidResult.Success:
            self._initialized = True
        return result
    
    def cleanup(self) -> VHidResult:
        """Cleans up the virtual device system's manager."""
        result = VHidResult(self._dll.vhid_cleanup())
        if result == VHidResult.Success:
            self._initialized = False
        return result

    def power_on(self) -> VHidResult:
        """Creates or finds the virtual devices, making them ready for input."""
        if not self._initialized: return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_power_on())

    def power_off(self) -> VHidResult:
        """Removes the virtual devices from the system."""
        if not self._initialized: return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_power_off())

    def reset_state(self) -> VHidResult:
        """Resets all internal states and sends 'all up' reports."""
        if not self._initialized: return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_reset_state())

    def send_mouse_report(self, report: MouseInput) -> VHidResult:
        """Sends a complete mouse report. Requires power_on() to have been called."""
        if not self._initialized: return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_send_mouse_report(ctypes.byref(report)))

    def send_keyboard_report(self, report: KeyboardInput) -> VHidResult:
        """Sends a complete keyboard report. Requires power_on() to have been called."""
        if not self._initialized: return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_send_keyboard_report(ctypes.byref(report)))

    def move_mouse_absolute(self, x: int, y: int) -> VHidResult:
        """Moves the mouse to absolute screen coordinates."""
        if not self._initialized: return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_mouse_move_absolute(x, y))

    def move_mouse(self, x: int, y: int) -> VHidResult:
        """Moves the mouse by a relative offset."""
        if not self._initialized: return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_mouse_move(x, y))
    
    def mouse_down(self, button: MouseButtons) -> VHidResult:
        """Presses a mouse button."""
        if not self._initialized: return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_mouse_down(button.value))

    def mouse_up(self, button: MouseButtons) -> VHidResult:
        """Releases a mouse button."""
        if not self._initialized: return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_mouse_up(button.value))

    def mouse_click(self, button: MouseButtons) -> VHidResult:
        """Performs a mouse click (down and up)."""
        if not self._initialized: return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_mouse_click(button.value))
    
    def mouse_wheel(self, delta: int) -> VHidResult:
        """Scrolls the mouse wheel."""
        if not self._initialized: return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_mouse_wheel(delta))

    def key_down(self, key_code: int) -> VHidResult:
        """Presses a keyboard key."""
        if not self._initialized: return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_key_down(key_code))

    def key_up(self, key_code: int) -> VHidResult:
        """Releases a keyboard key."""
        if not self._initialized: return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_key_up(key_code))

    def modifier_down(self, modifier: KeyModifiers) -> VHidResult:
        """Presses a modifier key (Ctrl, Shift, etc.)."""
        if not self._initialized: return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_modifier_down(modifier.value))

    def modifier_up(self, modifier: KeyModifiers) -> VHidResult:
        """Releases a modifier key."""
        if not self._initialized: return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_modifier_up(modifier.value))
    
    def key_tap(self, key_code: int) -> VHidResult:
        """Presses and releases a key using the native library function."""
        if not self._initialized: return VHidResult.NotInitialized
        return VHidResult(self._dll.vhid_key_tap(key_code))

    def get_last_error(self) -> str:
        """Gets the last error message from the library."""
        buffer = ctypes.create_string_buffer(256)
        length = self._dll.vhid_get_last_error(buffer, 256)
        if length > 0:
            return buffer.value.decode('utf-8', errors='ignore')
        return "Unknown error"
    
    def is_initialized(self) -> bool:
        """Checks if the library is initialized."""
        return self._initialized

# HID Usage Page 7: Keyboard/Keypad
class KeyCodes:
    A = 0x04; B = 0x05; C = 0x06; D = 0x07; E = 0x08; F = 0x09
    G = 0x0A; H = 0x0B; I = 0x0C; J = 0x0D; K = 0x0E; L = 0x0F
    M = 0x10; N = 0x11; O = 0x12; P = 0x13; Q = 0x14; R = 0x15
    S = 0x16; T = 0x17; U = 0x18; V = 0x19; W = 0x1A; X = 0x1B
    Y = 0x1C; Z = 0x1D
    NUM_1 = 0x1E; NUM_2 = 0x1F; NUM_3 = 0x20; NUM_4 = 0x21; NUM_5 = 0x22
    NUM_6 = 0x23; NUM_7 = 0x24; NUM_8 = 0x25; NUM_9 = 0x26; NUM_0 = 0x27
    ENTER = 0x28; ESC = 0x29; BACKSPACE = 0x2A; TAB = 0x2B; SPACE = 0x2C
    MINUS = 0x2D; EQUALS = 0x2E; LEFT_BRACKET = 0x2F; RIGHT_BRACKET = 0x30
    BACKSLASH = 0x31; SEMICOLON = 0x33; QUOTE = 0x34; GRAVE = 0x35
    COMMA = 0x36; PERIOD = 0x37; SLASH = 0x38; CAPS_LOCK = 0x39
    F1 = 0x3A; F2 = 0x3B; F3 = 0x3C; F4 = 0x3D; F5 = 0x3E; F6 = 0x3F
    F7 = 0x40; F8 = 0x41; F9 = 0x42; F10 = 0x43; F11 = 0x44; F12 = 0x45
