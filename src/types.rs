use std::mem::size_of;

/// 鼠标输入结构体
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MouseInput {
    pub button: i8,   // 按钮状态 (0=无, 1=左键, 2=右键, 3=中键)
    pub x: i8,        // X轴移动 (-127 到 127)
    pub y: i8,        // Y轴移动 (-127 到 127)
    pub wheel: i8,    // 滚轮移动
    pub reserved: i8, // 保留字段
}

impl MouseInput {
    pub fn new() -> Self {
        Self {
            button: 0,
            x: 0,
            y: 0,
            wheel: 0,
            reserved: 0,
        }
    }

    pub fn with_movement(x: i8, y: i8) -> Self {
        Self {
            x,
            y,
            ..Self::new()
        }
    }

    pub fn with_button(button: i8) -> Self {
        Self {
            button,
            ..Self::new()
        }
    }

    pub fn with_wheel(wheel: i8) -> Self {
        Self {
            wheel,
            ..Self::new()
        }
    }
}

/// 键盘输入结构体
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct KeyboardInput {
    pub modifiers: u8, // 修饰键 (Ctrl, Alt, Shift等)
    pub reserved: u8,  // 保留字段
    pub keys: [u8; 6], // 按键码数组 (最多6个按键)
}

impl KeyboardInput {
    pub fn new() -> Self {
        Self {
            modifiers: 0,
            reserved: 0,
            keys: [0; 6],
        }
    }

    pub fn with_key(key: u8) -> Self {
        let mut input = Self::new();
        input.keys[0] = key;
        input
    }

    pub fn with_modifiers(modifiers: u8, keys: [u8; 6]) -> Self {
        Self {
            modifiers,
            reserved: 0,
            keys,
        }
    }

    pub fn release_all() -> Self {
        Self::new()
    }
}

/// 设备ID存储结构
#[derive(Debug, Clone)]
pub struct DeviceIds {
    pub keyboard_id: Option<u32>,
    pub mouse_id: Option<u32>,
}

impl DeviceIds {
    pub fn new() -> Self {
        Self {
            keyboard_id: None,
            mouse_id: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.keyboard_id.is_none() && self.mouse_id.is_none()
    }

    pub fn has_keyboard(&self) -> bool {
        self.keyboard_id.is_some()
    }

    pub fn has_mouse(&self) -> bool {
        self.mouse_id.is_some()
    }
}
