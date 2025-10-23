#!/usr/bin/env python3
"""
关闭所有OpenCV窗口
用于清理残留的预览窗口
"""
import cv2
import os
import signal
import sys


def close_all_windows():
    """关闭所有OpenCV窗口"""
    try:
        # 关闭所有OpenCV窗口
        cv2.destroyAllWindows()
        print("✅ 已关闭所有OpenCV窗口")
    except Exception as e:
        print(f"❌ 关闭窗口时出错: {e}")


def kill_opencv_processes():
    """终止所有包含opencv的Python进程"""
    try:
        import subprocess
        
        # 查找包含opencv的Python进程
        result = subprocess.run(['pgrep', '-f', 'python.*opencv'], 
                              capture_output=True, text=True)
        
        if result.stdout.strip():
            pids = result.stdout.strip().split('\n')
            for pid in pids:
                if pid.strip():
                    try:
                        os.kill(int(pid), signal.SIGTERM)
                        print(f"✅ 已终止进程 PID: {pid}")
                    except Exception as e:
                        print(f"❌ 终止进程 {pid} 时出错: {e}")
        else:
            print("ℹ️ 没有找到包含opencv的Python进程")
            
    except Exception as e:
        print(f"❌ 查找进程时出错: {e}")


def main():
    """主函数"""
    print("🧹 清理OpenCV窗口和进程...")
    
    # 关闭所有窗口
    close_all_windows()
    
    # 终止相关进程
    kill_opencv_processes()
    
    print("✅ 清理完成!")


if __name__ == '__main__':
    main()




