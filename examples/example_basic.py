import sys
import os
import time

# Add the parent directory to the system path to find the logi_vhid module
current_path = os.path.abspath(__file__)
parent_dir = os.path.dirname(os.path.dirname(current_path))
sys.path.append(parent_dir)

from logi_vhid import LogiVHid, KeyCodes, MouseButtons, VHidResult

def main():
    # Create the virtual HID manager
    vhid = LogiVHid()
    
    try:
        # Initialize the system
        print("Initializing virtual device system...")
        result = vhid.initialize()
        if result != VHidResult.Success:
            print(f"Initialization failed: {vhid.get_last_error()}")
            return
        print("System initialized successfully!")

        # Power on the devices
        print("Powering on virtual devices...")
        result = vhid.power_on()
        if result != VHidResult.Success:
            print(f"Power on failed: {vhid.get_last_error()}")
            vhid.cleanup()
            return
        
        print("Waiting for virtual devices to be ready...")
        time.sleep(2)
        
        # --- Demonstrate Mouse Control ---
        print("\nDemonstrating Mouse Control...")
        print("Moving mouse relative (10, 5) then (-10, -5)...")
        vhid.move_mouse(10, 5)
        time.sleep(0.5)
        vhid.move_mouse(-10, -5)
        time.sleep(0.5)
        
        print("Clicking left mouse button...")
        vhid.mouse_click(MouseButtons.LEFT)
        time.sleep(0.5)
        
        print("Scrolling mouse wheel up and down...")
        vhid.mouse_wheel(5)   # Scroll up
        time.sleep(0.5)
        vhid.mouse_wheel(-5)  # Scroll down
        
        # --- Demonstrate Keyboard Control ---
        print("\nDemonstrating Keyboard Control...")
        print("Typing 'abc' followed by Enter...")
        vhid.key_tap(KeyCodes.A)
        time.sleep(0.2)
        vhid.key_tap(KeyCodes.B)
        time.sleep(0.2)
        vhid.key_tap(KeyCodes.C)
        time.sleep(0.2)
        vhid.key_tap(KeyCodes.ENTER)
        
        # --- Demonstrate State Reset ---
        print("\nDemonstrating Reset State...")
        print("Pressing left mouse button and 'a' key down (will not release)...")
        vhid.mouse_down(MouseButtons.LEFT)
        vhid.key_down(KeyCodes.A)
        time.sleep(1)
        print("Calling reset_state() to release all...")
        vhid.reset_state()
        time.sleep(1)
        print("State has been reset.")
        
        print("\nDemonstration complete!")
        
    except Exception as e:
        print(f"An unexpected error occurred: {e}")
    
    finally:
        # Clean up resources
        if vhid.is_initialized():
            print("\nPowering off devices...")
            vhid.power_off()
            print("Cleaning up resources...")
            vhid.cleanup()
            print("Cleanup finished.")

if __name__ == "__main__":
    main()
