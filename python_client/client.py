#!/usr/bin/env python3
"""
LoadExc Python 客户端
功能：订阅控制指令话题，发布带有控制指令的视频画面，带预览功能
"""
import rclpy
from rclpy.node import Node
from rclpy.qos import QoSProfile, ReliabilityPolicy, DurabilityPolicy
from std_msgs.msg import String
from sensor_msgs.msg import Image
from cv_bridge import CvBridge
import cv2
import numpy as np
import threading
import time
import os
import argparse
from queue import Queue, Empty
from typing import Optional

from control_message import UnifiedControlMessage
from video_renderer import VideoRenderer


class LoadExcClient(Node):
    """LoadExc Python 客户端节点"""
    
    def __init__(self, preview_width=1280, preview_height=720, enable_preview=True):
        super().__init__('load_exc_client')
        
        # 初始化
        self.bridge = CvBridge()
        self.renderer = VideoRenderer()
        
        # 控制状态
        self.control_data = UnifiedControlMessage()
        self.control_lock = threading.Lock()
        
        # 视频发布队列
        self.video_queue = Queue(maxsize=10)
        
        # 预览窗口设置
        self.show_preview = enable_preview
        self.preview_window_name = 'LoadExc Video Stream Preview'
        self.preview_width = preview_width
        self.preview_height = preview_height
        
        # 设置预览窗口
        if self.show_preview:
            self.setup_preview_window()
        
        # QoS 配置
        qos_profile = QoSProfile(
            reliability=ReliabilityPolicy.BEST_EFFORT,
            durability=DurabilityPolicy.VOLATILE,
            depth=1
        )
        
        # 创建订阅者 - 接收控制消息
        self.control_subscription = self.create_subscription(
            String,
            '/controls/teleop',
            self.control_callback,
            qos_profile
        )
        
        # 创建发布者 - 发布视频流
        self.video_publisher = self.create_publisher(
            Image,
            '/camera_front_wide',
            qos_profile
        )
        
        # 创建定时器 - 定期发布视频帧
        self.timer = self.create_timer(1.0/30.0, self.publish_video_frame)  # 30 FPS
        
        self.get_logger().info('🚀 LoadExc Python 客户端启动成功!')
        self.get_logger().info('📡 订阅控制话题: /controls/teleop')
        self.get_logger().info('📷 发布视频话题: /camera_front_wide')
        if self.show_preview:
            self.get_logger().info(f'🖥️ 预览窗口: {preview_width}x{preview_height}')
        else:
            self.get_logger().info('🖥️ 预览窗口已禁用')
    
    def setup_preview_window(self):
        """设置预览窗口"""
        try:
            cv2.namedWindow(self.preview_window_name, cv2.WINDOW_NORMAL)
            cv2.resizeWindow(self.preview_window_name, self.preview_width, self.preview_height)
            self.get_logger().info(f'📏 设置预览窗口大小: {self.preview_width}x{self.preview_height}')
        except Exception as e:
            self.get_logger().error(f'❌ 设置预览窗口失败: {e}')
            self.show_preview = False
    
    def control_callback(self, msg: String):
        """处理接收到的控制消息"""
        try:
            # 创建新的控制消息对象
            new_control = UnifiedControlMessage()
            
            # 解析 JSON 控制消息
            if new_control.update_from_json(msg.data):
                with self.control_lock:
                    # 更新控制状态
                    self.control_data = new_control
                
                self.get_logger().info(f'📥 收到控制消息: {msg.data[:100]}...')
            else:
                self.get_logger().warn('⚠️ 控制消息解析失败')
                
        except Exception as e:
            self.get_logger().error(f'❌ 处理控制消息时出错: {e}')
    
    def publish_video_frame(self):
        """发布视频帧"""
        try:
            with self.control_lock:
                current_control = self.control_data
            
            # 确保时间戳一致
            current_time = int(time.time() * 1000)
            current_control.timestamp = current_time
            
            # 渲染用于推流的视频帧（不包含延迟信息）
            frame = self.renderer.render_frame_for_streaming(current_control)
            
            # 转换为 ROS2 Image 消息
            try:
                # 转换为 I420 格式发布（与 Rust 客户端兼容）
                # 将 BGR 转换为 YUV420
                yuv_frame = cv2.cvtColor(frame, cv2.COLOR_BGR2YUV_I420)
                
                # 手动创建 ROS2 Image 消息，避免 cv_bridge 的编码问题
                from sensor_msgs.msg import Image
                ros_image = Image()
                ros_image = Image()
                ros_image.header.stamp = self.get_clock().now().to_msg()
                ros_image.header.frame_id = 'camera_link'
                ros_image.height = frame.shape[0]
                ros_image.width = frame.shape[1]
                ros_image.encoding = 'i420'
                ros_image.is_bigendian = False
                ros_image.step = yuv_frame.shape[1]  # 每行字节数
                ros_image.data = yuv_frame.tobytes()
                
                # 发布视频帧
                self.video_publisher.publish(ros_image)
                
                # 显示本地预览 - 显示与推流完全相同的画面
                if self.show_preview:
                    try:
                        # 直接显示原始BGR帧，确保预览和推流画面完全一致
                        display_frame = cv2.resize(frame, (self.preview_width, self.preview_height))
                        cv2.imshow(self.preview_window_name, display_frame)
                        cv2.waitKey(1)
                    except Exception as e:
                        self.get_logger().error(f'❌ 显示预览时出错: {e}')
                        self.show_preview = False
                
            except Exception as e:
                self.get_logger().error(f'❌ 发布视频帧时出错: {e}')
                
        except Exception as e:
            self.get_logger().error(f'❌ 渲染视频帧时出错: {e}')
    
    def resize_preview_window(self, width: int, height: int):
        """调整预览窗口大小"""
        if self.show_preview:
            self.preview_width = width
            self.preview_height = height
            try:
                cv2.resizeWindow(self.preview_window_name, width, height)
                self.get_logger().info(f'📏 调整预览窗口大小: {width}x{height}')
            except Exception as e:
                self.get_logger().error(f'❌ 调整窗口大小时出错: {e}')
    
    def toggle_preview(self):
        """切换预览窗口显示"""
        self.show_preview = not self.show_preview
        if self.show_preview:
            self.setup_preview_window()
            self.get_logger().info('🖥️ 启用预览窗口')
        else:
            try:
                cv2.destroyWindow(self.preview_window_name)
                self.get_logger().info('🖥️ 禁用预览窗口')
            except:
                pass
    
    def get_control_status(self) -> dict:
        """获取当前控制状态"""
        with self.control_lock:
            return self.control_data.to_dict()
    
    def close_preview(self):
        """关闭预览窗口"""
        if self.show_preview:
            try:
                cv2.destroyWindow(self.preview_window_name)
                self.get_logger().info('🖥️ 关闭预览窗口')
            except:
                pass


def main():
    """主函数"""
    parser = argparse.ArgumentParser(description='LoadExc Python Client')
    parser.add_argument('--width', type=int, default=1280, help='Preview window width (default: 1280)')
    parser.add_argument('--height', type=int, default=720, help='Preview window height (default: 720)')
    parser.add_argument('--no-preview', action='store_true', help='Disable preview window')
    
    args = parser.parse_args()
    
    print("🚀 启动 LoadExc Python 客户端...")
    print(f"📏 预览窗口大小: {args.width}x{args.height}")
    
    # 初始化 ROS2
    rclpy.init()
    
    try:
        # 创建客户端节点
        client = LoadExcClient(
            preview_width=args.width, 
            preview_height=args.height,
            enable_preview=not args.no_preview
        )
        
        print("✅ 客户端启动成功!")
        print("📋 使用说明:")
        print("   - 控制话题: /controls/teleop")
        print("   - 视频话题: /camera_front_wide")
        if not args.no_preview:
            print("   - 预览窗口: 可调整大小")
        print("   - 按 Ctrl+C 退出")
        print()
        
        # 启动 ROS2 节点
        rclpy.spin(client)
        
    except KeyboardInterrupt:
        print("\n🛑 收到退出信号...")
    except Exception as e:
        print(f"❌ 运行时出错: {e}")
    finally:
        # 清理
        if 'client' in locals():
            client.close_preview()
            client.destroy_node()
        rclpy.shutdown()
        cv2.destroyAllWindows()
        print("✅ 程序已退出")


if __name__ == '__main__':
    main()
