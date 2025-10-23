#!/usr/bin/env python3
"""
测试 ROS2 连接和话题通信
用于验证 Rust 和 Python 客户端之间的通信
"""

import rclpy
from rclpy.node import Node
from std_msgs.msg import String
from sensor_msgs.msg import Image
import json
import time
import threading
from queue import Queue

class ROS2ConnectionTester(Node):
    """ROS2 连接测试器"""
    
    def __init__(self):
        super().__init__('ros2_connection_tester')
        
        # 创建发布者 - 发送测试控制消息
        self.control_publisher = self.create_publisher(
            String,
            '/controls/teleop',
            10
        )
        
        # 创建订阅者 - 接收视频消息
        self.video_subscription = self.create_subscription(
            Image,
            '/camera_front_wide',
            self.video_callback,
            10
        )
        
        # 消息计数器
        self.control_count = 0
        self.video_count = 0
        
        self.get_logger().info('🧪 ROS2 连接测试器启动')
    
    def video_callback(self, msg: Image):
        """接收视频消息的回调"""
        self.video_count += 1
        if self.video_count % 30 == 0:  # 每30帧打印一次
            self.get_logger().info(f'📷 收到视频帧 #{self.video_count} (尺寸: {msg.width}x{msg.height})')
    
    def send_test_control_message(self, msg_type: str = "analog"):
        """发送测试控制消息"""
        self.control_count += 1
        
        if msg_type == "gear":
            # 发送档位消息
            test_msg = {
                "type": "gear",
                "gear": "D" if self.control_count % 2 == 0 else "R",
                "t": int(time.time() * 1000)
            }
        else:
            # 发送模拟控制消息
            import math
            test_msg = {
                "type": "analog",
                "v": {
                    "rotation": 0.5 * math.sin(time.time()),
                    "brake": 0.0,
                    "throttle": 0.3 + 0.2 * math.cos(time.time() * 0.5),
                    "boom": 0.1 * math.sin(time.time() * 0.3),
                    "bucket": 0.1 * math.cos(time.time() * 0.3)
                },
                "t": int(time.time() * 1000)
            }
        
        # 发布消息
        ros_msg = String()
        ros_msg.data = json.dumps(test_msg)
        self.control_publisher.publish(ros_msg)
        
        self.get_logger().info(f'📤 发送控制消息 #{self.control_count}: {json.dumps(test_msg, indent=2)}')
    
    def run_test(self, duration: int = 30):
        """运行测试"""
        self.get_logger().info(f'🚀 开始 {duration} 秒的 ROS2 连接测试...')
        
        start_time = time.time()
        message_interval = 1.0  # 每秒发送一次消息
        
        while time.time() - start_time < duration:
            # 发送测试消息
            self.send_test_control_message("analog")
            time.sleep(message_interval)
            
            # 每5秒发送一次档位消息
            if int(time.time() - start_time) % 5 == 0:
                self.send_test_control_message("gear")
        
        self.get_logger().info(f'✅ 测试完成! 发送了 {self.control_count} 条控制消息，收到 {self.video_count} 个视频帧')

def main():
    """主函数"""
    print("🧪 ROS2 连接测试器")
    print("=" * 50)
    
    # 初始化 ROS2
    rclpy.init()
    
    try:
        # 创建测试器
        tester = ROS2ConnectionTester()
        
        print("📋 测试说明:")
        print("   - 发送控制消息到 /controls/teleop")
        print("   - 接收视频消息从 /camera_front_wide")
        print("   - 运行 30 秒测试")
        print("   - 按 Ctrl+C 提前退出")
        print()
        
        # 启动测试线程
        test_thread = threading.Thread(
            target=tester.run_test,
            args=(30,),
            daemon=True
        )
        test_thread.start()
        
        # 运行 ROS2 节点
        rclpy.spin(tester)
        
    except KeyboardInterrupt:
        print("\n🛑 测试被用户中断")
    except Exception as e:
        print(f"❌ 测试出错: {e}")
    finally:
        rclpy.shutdown()
        print("✅ 测试器已退出")

if __name__ == '__main__':
    main()






