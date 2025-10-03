# Logi V-HID Manager - 罗技虚拟HID输入管理器

**⚠️ 免责声明 (Disclaimer) ⚠️**

**本项目仅限于技术交流和学习研究目的，严禁用于任何商业或非法用途。**

本软件通过利用一个已公开的旧版罗技驱动程序漏洞 (CVE) 来实现其功能。作者不对您使用此软件所造成的任何后果负责，包括但不限于：数据丢失、账户封禁、硬件损坏或任何形式的法律责任。

**您必须自行承担使用此软件的所有风险。在任何有反作弊措施的环境（如在线游戏）中使用此类工具，几乎必然会导致您的账户受到永久封禁等严厉处罚。**

本项目与罗技 (Logitech) 公司没有任何关联。

---

**项目原理**:
本项目利用了旧版罗技游戏软件（Logitech Gaming Software）驱动中存在的已知漏洞（CVE）。通过与该驱动进行交互，我们可以在内核中枚举出一个虚拟总线，并在这个总线上挂载我们自定义的虚拟键盘和虚拟鼠标。所有通过这些虚拟设备发送的输入，都将被操作系统视为真实的物理硬件输入。

关于此漏洞的原始研究，请参考：[logitech-cve by ekknod](https://github.com/ekknod/logitech-cve)

## 主要特性

* **驱动级输入**: 实现 `SendInput` 等 API 更底层的输入模拟。
* **独立的命令行工具**: 提供 `install`, `uninstall`, `create-hid`, `destroy-hid` 等命令，用于管理驱动和虚拟设备的整个生命周期。
* **底层调试接口**: 命令行工具提供 `mouse-report` 和 `keyboard-report` 命令，用于发送原始的 HID 报告，方便调试和测试。
* **跨语言动态库 (DLL)**: 项目可被编译为一个标准的 C ABI 动态链接库 (`.dll`)，方便被 Python, C#, C++ 等其他语言调用。
* **智能化的库接口**: 提供的库函数是**有状态的**，能够自动管理鼠标按键的按下状态，轻松实现拖拽等复杂操作。
* **精准的绝对位置移动**: 库提供了 `vhid_mouse_move_absolute` 函数，通过优化的迭代算法，可实现精准、平滑的屏幕绝对坐标移动。

## 使用说明

### 1. 环境准备

* **安装驱动**: logi_vhid_manager.exe install。
* **鼠标设置**: 为了获得最佳的绝对定位精度，请在使用前进行以下 Windows 设置：
    1.  进入“鼠标设置”。
    2.  点击“其他鼠标选项”。
    3.  在“指针选项”卡中，**取消勾选“提高指针精确度”**。
    4.  将“选择指针移动速度”滑块精确地移动到**正中间（第6格）**。

### 2. 命令行工具 (`main.exe`) 用法

命令行工具主要用于驱动管理和底层调试。

```sh
# (以管理员身份运行)

# 1. 安装驱动和设备
logi_vhid_manager.exe install

# 2. 创建虚拟键鼠
logi_vhid_manager.exe create-hid

# 3. (可选) 发送底层鼠标报告进行测试
#    格式: mouse-report <button> <x> <y> <wheel>
#    按下左键(1)并向右移动10像素
logi_vhid_manager.exe mouse-report 1 10 0 0

# 4. (可选) 发送底层键盘报告进行测试
#    格式: keyboard-report <modifiers> [key1]..[key6] (十六进制)
#    按下 'A' 键 (HID code 0x04)
logi_vhid_manager.exe keyboard-report 0 04

# 5. 销毁虚拟键鼠
logi_vhid_manager.exe destroy-hid

# 6. 卸载驱动和设备
logi_vhid_manager.exe uninstall
```

### 3. Python 库调用 (`lib.dll`) 示例

这是推荐的、用于自动化脚本编程的方式。

```python
import time
from logi_vhid import LogiVHid, MouseButtons, KeyCodes, VHidResult

# 实例化管理器，它会自动找到 .dll 文件
vhid = LogiVHid()

try:
    # 1. 初始化，与驱动建立连接
    if vhid.initialize() != VHidResult.Success:
        raise RuntimeError(f"初始化失败: {vhid.get_last_error()}")

    # 2. "打开"虚拟键鼠
    if vhid.power_on() != VHidResult.Success:
        raise RuntimeError(f"激活设备失败: {vhid.get_last_error()}")
    
    print("设备已就绪，2秒后开始演示...")
    time.sleep(2)

    # 3. 执行操作 (例如，拖拽出一个正方形)
    print("正在绘制正方形...")
    vhid.mouse_move_absolute(200, 200) # 移动到起点
    time.sleep(0.5)
    
    vhid.mouse_down(MouseButtons.LEFT) # 按下左键
    time.sleep(0.2)

    # 因为库是状态化的，move_absolute 会自动保持按下状态
    vhid.mouse_move_absolute(400, 200)
    vhid.mouse_move_absolute(400, 400)
    vhid.mouse_move_absolute(200, 400)
    vhid.mouse_move_absolute(200, 200)
    
    time.sleep(0.2)
    vhid.mouse_up(MouseButtons.LEFT) # 释放左键
    
    print("绘制完成!")
    time.sleep(1)

    # 演示键盘输入
    print("正在输入'hello'...")
    vhid.key_tap(KeyCodes.H)
    vhid.key_tap(KeyCodes.E)
    vhid.key_tap(KeyCodes.L)
    vhid.key_tap(KeyCodes.L)
    vhid.key_tap(KeyCodes.O)

except Exception as e:
    print(f"\n[!] 发生错误: {e}")

finally:
    # 4. 确保资源被正确清理
    if vhid.is_initialized():
        print("\n[*] 正在清理资源...")
        vhid.reset_state()      # 重置所有按键状态
        vhid.power_off()        # "关闭"虚拟设备
        vhid.cleanup()          # 断开与驱动的连接
        print("[*] 清理完成。")

```