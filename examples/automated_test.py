import sys
import os
import time
import unittest
import ctypes
from ctypes import wintypes, c_uint8

# --- WinAPI 定义 ---
# 用于与 Windows API 交互以验证真实输入效果

class POINT(ctypes.Structure):
    _fields_ = [("x", wintypes.LONG), ("y", wintypes.LONG)]

# 定义虚拟键码 (Virtual-Key Codes)
VK_LBUTTON = 0x01  # 鼠标左键
VK_SHIFT = 0x10    # Shift 键
VK_CAPITAL = 0x14  # Caps Lock 键

user32 = ctypes.WinDLL('user32', use_last_error=True)

def get_cursor_pos() -> POINT:
    """获取当前鼠标光标的屏幕坐标"""
    pt = POINT()
    user32.GetCursorPos(ctypes.byref(pt))
    return pt

def get_key_state(vk_code: int) -> int:
    """获取指定虚拟键的状态"""
    return user32.GetKeyState(vk_code)

def is_key_pressed(vk_code: int) -> bool:
    """检查一个键当前是否被按下（高位为1）"""
    return (get_key_state(vk_code) & 0x8000) != 0

def is_key_toggled(vk_code: int) -> bool:
    """检查一个键是否处于切换状态（低位为1），例如 Caps Lock"""
    return (get_key_state(vk_code) & 1) != 0

# --- 测试代码 ---

# 将父目录添加到系统路径
current_path = os.path.abspath(__file__)
parent_dir = os.path.dirname(os.path.dirname(current_path))
sys.path.append(parent_dir)

from logi_vhid import LogiVHid, KeyCodes, KeyModifiers, MouseButtons, VHidResult, MouseInput, KeyboardInput

class TestLogiVHid(unittest.TestCase):
    """
    针对 LogiVHid 库的自动化测试套件。
    这个测试会通过调用 Windows API 来验证输入事件是否真实有效。
    """
    vhid = None
    
    @classmethod
    def setUpClass(cls):
        """在所有测试开始前运行一次，用于初始化系统。"""
        print("---  初始化测试环境 ---")
        cls.vhid = LogiVHid()
        
        print("[1/3] 初始化虚拟设备系统...")
        result = cls.vhid.initialize()
        if result != VHidResult.Success:
            raise Exception(f"初始化失败: {cls.vhid.get_last_error()}")
        
        print("[2/3] 启动虚拟设备...")
        result = cls.vhid.power_on()
        if result != VHidResult.Success:
            cls.vhid.cleanup()
            raise Exception(f"启动设备失败: {cls.vhid.get_last_error()}")
            
        print("[3/3] 等待操作系统识别设备...")
        time.sleep(3) # 等待足够的时间以确保设备就绪
        print("--- 环境准备就绪 ---")

    @classmethod
    def tearDownClass(cls):
        """在所有测试结束后运行一次，用于清理资源。"""
        if cls.vhid and cls.vhid.is_initialized():
            print("\n--- 开始清理测试环境 ---")
            print("[1/2] 关闭虚拟设备...")
            cls.vhid.power_off()
            print("[2/2] 清理系统资源...")
            cls.vhid.cleanup()
            print("--- 环境清理完毕 ---")

    def setUp(self):
        """在每个测试方法执行前运行，用于重置状态。"""
        self.vhid.reset_state()
        time.sleep(0.05)

    def test_01_mouse_move_relative_real(self):
        """测试鼠标相对移动的真实效果"""
        print("\n测试: 验证鼠标相对移动...")
        start_pos = get_cursor_pos()
        move_x, move_y = 20, -15
        tolerance = 10  # 允许 10 像素的误差

        result = self.vhid.move_mouse(move_x, move_y)
        self.assertEqual(result, VHidResult.Success)
        time.sleep(0.1) # 等待系统处理移动

        end_pos = get_cursor_pos()
        
        expected_x = start_pos.x + move_x
        expected_y = start_pos.y + move_y

        self.assertTrue(abs(end_pos.x - expected_x) <= tolerance, f"鼠标 X 轴相对移动超出容差范围: 期望值 {expected_x}, 实际值 {end_pos.x}")
        self.assertTrue(abs(end_pos.y - expected_y) <= tolerance, f"鼠标 Y 轴相对移动超出容差范围: 期望值 {expected_y}, 实际值 {end_pos.y}")
        print("通过")

    def test_02_mouse_move_absolute_real(self):
        """测试鼠标绝对移动的真实效果"""
        print("\n测试: 验证鼠标绝对移动...")
        target_x, target_y = 250, 250
        tolerance = 10  # 允许 10 像素的误差

        result = self.vhid.move_mouse_absolute(target_x, target_y)
        self.assertEqual(result, VHidResult.Success)
        time.sleep(0.2) # 绝对移动可能需要更长的时间

        final_pos = get_cursor_pos()

        self.assertTrue(abs(final_pos.x - target_x) <= tolerance, f"鼠标 X 轴绝对移动超出容差范围: 期望值 {target_x}, 实际值 {final_pos.x}")
        self.assertTrue(abs(final_pos.y - target_y) <= tolerance, f"鼠标 Y 轴绝对移动超出容差范围: 期望值 {target_y}, 实际值 {final_pos.y}")
        print("通过")

    def test_03_mouse_button_state_real(self):
        """测试鼠标按键是否改变系统状态"""
        print("\n测试: 验证鼠标按键状态...")
        self.assertFalse(is_key_pressed(VK_LBUTTON), "测试开始前鼠标左键应为抬起状态")
        
        self.vhid.mouse_down(MouseButtons.LEFT)
        time.sleep(0.1)
        self.assertTrue(is_key_pressed(VK_LBUTTON), "mouse_down 后系统未检测到左键按下")

        self.vhid.mouse_up(MouseButtons.LEFT)
        time.sleep(0.1)
        self.assertFalse(is_key_pressed(VK_LBUTTON), "mouse_up 后系统未检测到左键释放")
        print("通过")

    def test_04_keyboard_toggle_key_real(self):
        """测试切换键 (Caps Lock) 的真实效果"""
        print("\n测试: 验证键盘切换键 (Caps Lock)...")
        initial_caps_state = is_key_toggled(VK_CAPITAL)
        
        self.vhid.key_tap(KeyCodes.CAPS_LOCK)
        time.sleep(0.1) # 等待系统处理状态切换
        
        final_caps_state = is_key_toggled(VK_CAPITAL)
        
        self.assertNotEqual(initial_caps_state, final_caps_state, "Caps Lock 状态未切换")
        
        # 恢复原始状态
        self.vhid.key_tap(KeyCodes.CAPS_LOCK)
        time.sleep(0.1)
        self.assertEqual(initial_caps_state, is_key_toggled(VK_CAPITAL), "未能将 Caps Lock 恢复到原始状态")
        print("通过")

    def test_05_modifier_key_state_real(self):
        """测试修饰键 (Shift) 是否改变系统状态"""
        print("\n测试: 验证修饰键状态 (Shift)...")
        self.assertFalse(is_key_pressed(VK_SHIFT), "测试开始前 Shift 键应为抬起状态")
        
        self.vhid.modifier_down(KeyModifiers.LEFT_SHIFT)
        time.sleep(0.1)
        self.assertTrue(is_key_pressed(VK_SHIFT), "modifier_down 后系统未检测到 Shift 按下")

        self.vhid.modifier_up(KeyModifiers.LEFT_SHIFT)
        time.sleep(0.1)
        self.assertFalse(is_key_pressed(VK_SHIFT), "modifier_up 后系统未检测到 Shift 释放")
        print("通过")

    def test_06_combination_shift_click_real(self):
        """测试组合键 (Shift + Click) 的真实效果"""
        print("\n测试: 验证组合键 (Shift + Click)...")
        # 验证初始状态
        self.assertFalse(is_key_pressed(VK_SHIFT), "前置条件：Shift 键应为抬起状态")
        self.assertFalse(is_key_pressed(VK_LBUTTON), "前置条件：左键应为抬起状态")

        # 按下 Shift
        self.vhid.modifier_down(KeyModifiers.LEFT_SHIFT)
        time.sleep(0.1)
        self.assertTrue(is_key_pressed(VK_SHIFT), "组合键测试中，Shift 未被按下")

        # 在 Shift 按下时进行点击
        self.vhid.mouse_click(MouseButtons.LEFT)
        time.sleep(0.1)

        # 释放 Shift
        self.vhid.modifier_up(KeyModifiers.LEFT_SHIFT)
        time.sleep(0.1)
        
        # 验证最终状态
        self.assertFalse(is_key_pressed(VK_SHIFT), "组合键测试后，Shift 未被释放")
        self.assertFalse(is_key_pressed(VK_LBUTTON), "组合键测试后，左键未被释放")
        print("通过")
        
    def test_07_raw_reports_still_work(self):
        """测试原始报告发送功能是否依然正常"""
        print("\n测试: 原始报告发送...")
        # 测试键盘
        report_key_down = KeyboardInput(modifiers=0, keys=(c_uint8 * 6)(KeyCodes.Z, 0, 0, 0, 0, 0))
        self.assertEqual(self.vhid.send_keyboard_report(report_key_down), VHidResult.Success)
        time.sleep(0.1)
        report_key_up = KeyboardInput(modifiers=0, keys=(c_uint8 * 6)(0, 0, 0, 0, 0, 0))
        self.assertEqual(self.vhid.send_keyboard_report(report_key_up), VHidResult.Success)
        
        # 测试鼠标
        report_mouse_move = MouseInput(button=0, x=5, y=5, wheel=0)
        self.assertEqual(self.vhid.send_mouse_report(report_mouse_move), VHidResult.Success)
        print("通过")


if __name__ == '__main__':
    print("===================================================")
    print("=      LogiVHid 自动化功能与效果验证测试      =")
    print("===================================================")
    # 运行测试
    unittest.main(verbosity=2) # 使用更高的详细级别以显示测试名称

