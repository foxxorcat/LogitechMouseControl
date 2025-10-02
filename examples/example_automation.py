import sys
import os
import time

# 获取当前文件的绝对路径
current_path = os.path.abspath(__file__)
# 计算父目录（examples的上一级目录，即python目录）
parent_dir = os.path.dirname(os.path.dirname(current_path))
# 将父目录添加到系统路径
sys.path.append(parent_dir)

import time
from logi_vhid import LogiVHid, KeyCodes,VHidResult

class AutoController:
    def __init__(self):
        self.vhid = LogiVHid()
        self.initialized = False
    
    def start(self):
        """启动控制器"""
        print("启动虚拟HID控制器...")
        result = self.vhid.initialize()
        if result != VHidResult.Success:
            raise Exception(f"初始化失败: {self.vhid.get_last_error()}")
        
        result = self.vhid.create_devices()
        if result != VHidResult.Success:
            raise Exception(f"创建设备失败: {self.vhid.get_last_error()}")
        
        self.initialized = True
        time.sleep(2)  # 等待设备就绪
        print("控制器就绪")
    
    def stop(self):
        """停止控制器"""
        if self.initialized:
            print("停止控制器...")
            self.vhid.destroy_devices()
            self.vhid.cleanup()
            self.initialized = False
    
    def draw_square(self, size=50):
        """绘制正方形"""
        print(f"绘制 {size}x{size} 正方形")
        # 向右
        for i in range(size):
            self.vhid.move_mouse(1, 0)
            time.sleep(0.01)
        # 向下
        for i in range(size):
            self.vhid.move_mouse(0, 1)
            time.sleep(0.01)
        # 向左
        for i in range(size):
            self.vhid.move_mouse(-1, 0)
            time.sleep(0.01)
        # 向上
        for i in range(size):
            self.vhid.move_mouse(0, -1)
            time.sleep(0.01)
    
    def type_text(self, text):
        """输入文本"""
        print(f"输入文本: {text}")
        key_map = {
            'a': KeyCodes.A, 'b': KeyCodes.B, 'c': KeyCodes.C, 'd': KeyCodes.D,
            'e': KeyCodes.E, 'f': KeyCodes.F, 'g': KeyCodes.G, 'h': KeyCodes.H,
            'i': KeyCodes.I, 'j': KeyCodes.J, 'k': KeyCodes.K, 'l': KeyCodes.L,
            'm': KeyCodes.M, 'n': KeyCodes.N, 'o': KeyCodes.O, 'p': KeyCodes.P,
            'q': KeyCodes.Q, 'r': KeyCodes.R, 's': KeyCodes.S, 't': KeyCodes.T,
            'u': KeyCodes.U, 'v': KeyCodes.V, 'w': KeyCodes.W, 'x': KeyCodes.X,
            'y': KeyCodes.Y, 'z': KeyCodes.Z,
            '1': KeyCodes.NUM_1, '2': KeyCodes.NUM_2, '3': KeyCodes.NUM_3,
            '4': KeyCodes.NUM_4, '5': KeyCodes.NUM_5, '6': KeyCodes.NUM_6,
            '7': KeyCodes.NUM_7, '8': KeyCodes.NUM_8, '9': KeyCodes.NUM_9,
            '0': KeyCodes.NUM_0, ' ': KeyCodes.SPACE,
        }
        
        for char in text.lower():
            if char in key_map:
                self.vhid.key_tap(key_map[char])
                time.sleep(0.05)
    
    def run_demo(self):
        """运行演示"""
        print("开始自动化演示...")
        
        # 绘制图形
        self.draw_square(30)
        time.sleep(1)
        
        # 输入文本
        self.type_text("hello world")
        time.sleep(1)
        
        # 按下回车
        self.vhid.key_tap(KeyCodes.ENTER)
        time.sleep(0.5)
        
        print("演示完成!")

def main():
    controller = AutoController()
    
    try:
        controller.start()
        controller.run_demo()
        
        # 等待用户输入
        input("按Enter键继续...")
        
    except Exception as e:
        print(f"错误: {e}")
    
    finally:
        controller.stop()

if __name__ == "__main__":
    main()