use winapi::shared::minwindef::DWORD;

// ================================
// 文件和设备常量
// ================================

/// 总线驱动INF文件名
pub const INF_BUS_FILE: &str = "logi_joy_bus_enum.inf";
/// HID驱动INF文件名  
pub const INF_HID_FILE: &str = "logi_joy_vir_hid.inf";
/// 设备硬件ID
pub const HARDWARE_ID: &str = "root\\LGHUBVirtualBus";
/// 设备名称
pub const DEVICE_NAME: &str = "System";
/// 总线设备路径
pub const BUS_DEVICE_PATH: &str = "\\\\?\\root#system#0001#{dfbedcdb-2148-416d-9e4d-cecc2424128c}";
/// 临时设备ID存储文件
pub const TEMP_ID_FILE: &str = "temp_device_ids.json";

// ================================
// IOCTL 控制码
// ================================

/// 创建设备的IOCTL控制码
pub const IOCTL_BUS_CREATE_DEVICE: DWORD = 0x2A2000;
/// 销毁设备的IOCTL控制码
pub const IOCTL_BUS_DESTROY_DEVICE: DWORD = 0x2A2004;

/// 移动鼠标的IOCTL控制码
pub const IOCTL_MOVE_MOUSE: DWORD = 0x2A2010;
/// 发送键盘输入的IOCTL控制码
pub const IOCTL_SEND_KEYBOARD: DWORD = 0x2A200C;