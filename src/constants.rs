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

// ================================
// IOCTL 控制码 (基于驱动分析)
// ================================

/// 创建设备的IOCTL控制码
pub const IOCTL_BUS_CREATE_DEVICE: u32 = 0x2A2000;
/// 销毁设备的IOCTL控制码
pub const IOCTL_BUS_DESTROY_DEVICE: u32 = 0x2A2004;

/// 写入数据到主设备的IOCTL控制码
pub const IOCTL_WRITE_PRIMARY_DEVICE: u32 = 0x2A200C;
/// 写入数据到次设备的IOCTL控制码
pub const IOCTL_WRITE_SECONDARY_DEVICE: u32 = 0x2A2010;
/// 从第三设备读取数据的IOCTL控制码
pub const IOCTL_READ_TERTIARY_DEVICE: u32 = 0x2A203C;

/// 启动异步读取的IOCTL控制码
pub const IOCTL_START_ASYNC_READ: u32 = 0x2A2023;
/// 启动异步写入的IOCTL控制码
pub const IOCTL_START_ASYNC_WRITE: u32 = 0x2A2024;

// ================================
// 设备特定常量
// ================================

/// Logitech 厂商ID
pub const VENDOR_ID_LOGITECH: u16 = 0x046D;
/// 虚拟键盘产品ID
pub const PRODUCT_ID_VIRTUAL_KEYBOARD: u16 = 0xC232;
/// 虚拟鼠标产品ID
pub const PRODUCT_ID_VIRTUAL_MOUSE: u16 = 0xC231;

/// 键盘设备类型标识
pub const DEVICE_TYPE_KEYBOARD: u32 = 0;
/// 鼠标设备类型标识
pub const DEVICE_TYPE_MOUSE: u32 = 1;