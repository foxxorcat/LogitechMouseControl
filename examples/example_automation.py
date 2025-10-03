import sys
import os
import time

# Add the parent directory to the system path to find the logi_vhid module
current_path = os.path.abspath(__file__)
parent_dir = os.path.dirname(os.path.dirname(current_path))
sys.path.append(parent_dir)

from logi_vhid import LogiVHid, KeyCodes, VHidResult

class AutoController:
    """An example automation controller using LogiVHid."""
    
    def __init__(self):
        self.vhid = LogiVHid()
        self.initialized = False
    
    def start(self):
        """Starts the controller and initializes the virtual HID system."""
        print("Initializing Virtual HID system...")
        result = self.vhid.initialize()
        if result != VHidResult.Success:
            raise Exception(f"Initialization failed: {self.vhid.get_last_error()}")
        
        print("Powering on virtual devices...")
        result = self.vhid.power_on()
        if result != VHidResult.Success:
            self.vhid.cleanup() # Clean up even if power on fails
            raise Exception(f"Failed to power on devices: {self.vhid.get_last_error()}")

        self.initialized = True
        print("Controller ready. Waiting for OS to recognize devices...")
        time.sleep(2)  # Wait for the OS to recognize the new virtual devices
    
    def stop(self):
        """Stops the controller and cleans up resources."""
        if self.initialized:
            print("Powering off virtual devices...")
            self.vhid.power_off()
            print("Cleaning up system resources...")
            self.vhid.cleanup()
            self.initialized = False
    
    def draw_square(self, size=50):
        """Moves the mouse in a square pattern."""
        print(f"Drawing a {size}x{size} square with the mouse.")
        # Move right
        for _ in range(size):
            self.vhid.move_mouse(x=1, y=0)
            time.sleep(0.01)
        # Move down
        for _ in range(size):
            self.vhid.move_mouse(x=0, y=1)
            time.sleep(0.01)
        # Move left
        for _ in range(size):
            self.vhid.move_mouse(x=-1, y=0)
            time.sleep(0.01)
        # Move up
        for _ in range(size):
            self.vhid.move_mouse(x=0, y=-1)
            time.sleep(0.01)
    
    def type_text(self, text: str):
        """Types the given text using virtual key presses."""
        print(f"Typing text: {text}")
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
                # The native key_tap has a built-in delay, so this can be shorter
                time.sleep(0.05)
    
    def run_demo(self):
        """Runs a demonstration of the controller's capabilities."""
        print("Starting automation demo...")
        
        # Draw a shape with the mouse
        self.draw_square(30)
        time.sleep(1)
        
        # Type some text
        self.type_text("hello world")
        time.sleep(1)
        
        # Press Enter
        self.vhid.key_tap(KeyCodes.ENTER)
        time.sleep(0.5)
        
        print("Demo finished!")

def main():
    controller = AutoController()
    
    try:
        controller.start()
        controller.run_demo()
        
        input("Press Enter to exit...")
        
    except Exception as e:
        print(f"An error occurred: {e}")
    
    finally:
        controller.stop()

if __name__ == "__main__":
    main()
