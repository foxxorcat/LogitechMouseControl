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
from logi_vhid import LogiVHid, KeyCodes, MouseButtons,VHidResult

def main():
    # 创建虚拟HID管理器
    vhid = LogiVHid()
    
    try:
        # 初始化
        print("初始化虚拟设备系统...")
        result = vhid.initialize()
        if result != VHidResult.Success:
            print(f"初始化失败: {vhid.get_last_error()}")
            return
        
        # 创建设备
        print("创建虚拟HID设备...")
        result = vhid.create_devices()
        if result != VHidResult.Success:
            print(f"创建设备失败: {vhid.get_last_error()}")
            return
        
        print("设备创建成功!")
        
        # 等待设备就绪
        time.sleep(2)
        
        # 演示鼠标控制
        print("演示鼠标控制...")
        vhid.move_mouse(10, 5)
        time.sleep(0.5)
        vhid.move_mouse(-10, -5)
        time.sleep(0.5)
        
        vhid.mouse_click(MouseButtons.LEFT)
        time.sleep(0.5)
        
        vhid.mouse_wheel(5)  # 向上滚动
        time.sleep(0.5)
        vhid.mouse_wheel(-5)  # 向下滚动
        
        # 演示键盘控制
        print("演示键盘控制...")
        vhid.key_tap(KeyCodes.A)  # 按下并释放A键
        time.sleep(0.2)
        vhid.key_tap(KeyCodes.B)
        time.sleep(0.2)
        vhid.key_tap(KeyCodes.C)
        time.sleep(0.2)
        vhid.key_tap(KeyCodes.ENTER)
        
        print("演示完成!")
        
    except Exception as e:
        print(f"发生错误: {e}")
    
    finally:
        # 清理资源
        if vhid.is_initialized():
            print("清理资源...")
            vhid.destroy_devices()
            vhid.cleanup()

if __name__ == "__main__":
    main()