#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import cv2
import rclpy
from rclpy.node import Node
from sensor_msgs.msg import Image
from std_msgs.msg import String
from cv_bridge import CvBridge
import argparse
import sys
import time
# from datetime import datetime  # 已禁用以降低延迟
import threading
import queue
# import json  # 已禁用以降低延迟


def open_camera(camera_source):
    cap = cv2.VideoCapture(camera_source, cv2.CAP_V4L2)
    if cap.isOpened():
        return cap
    cap.release()
    if isinstance(camera_source, int):
        device_path = f"/dev/video{camera_source}"
        cap = cv2.VideoCapture(device_path, cv2.CAP_V4L2)
        if cap.isOpened():
            return cap
        cap.release()
        cap = cv2.VideoCapture(device_path)
        if cap.isOpened():
            return cap
    else:
        cap = cv2.VideoCapture(camera_source)
        if cap.isOpened():
            return cap
    return None


class LoadExcCameraPublisher(Node):
    def __init__(self, camera_source, topic, width, height, fps, display, encoding, fourcc):
        super().__init__('load_exc_camera_publisher')
        self.bridge = CvBridge()
        self.publisher = self.create_publisher(Image, topic, 10)
        self.display = display
        self.target_fps = fps
        self.frame_interval = 1.0 / fps if fps > 0 else 0.0
        self.frame_count = 0
        self.start_time = time.time()
        
        # 控制消息处理（已禁用以降低延迟）
        # self.control_data = {}
        # self.control_lock = threading.Lock()
        # self.control_subscription = self.create_subscription(
        #     String,
        #     '/controls/teleop',
        #     self.control_callback,
        #     10
        # )
        
        # 支持编码：bgr8/rgb8（CvBridge）与 i420（手动构造）
        self.encoding = encoding.lower()
        if self.encoding not in ('bgr8', 'rgb8', 'i420'):
            self.get_logger().warn(f"不支持的编码 {encoding}，已回退为 i420")
            self.encoding = 'i420'

        self.cap = open_camera(camera_source)
        if self.cap is None or not self.cap.isOpened():
            self.get_logger().error(f"无法打开摄像头: {camera_source}")
            self.get_logger().error("排查: 1) ls -l /dev/video* 2) lsof /dev/video* 3) 确保用户在 video 组")
            sys.exit(1)

        # 设置FOURCC（可提升高分辨率帧率，如 MJPG/YUYV/H264，取决于设备支持）
        if fourcc:
            try:
                fourcc_code = cv2.VideoWriter_fourcc(*fourcc)
                self.cap.set(cv2.CAP_PROP_FOURCC, fourcc_code)
                self.get_logger().info(f"已设置 FOURCC: {fourcc}")
            except Exception as e:
                self.get_logger().warn(f"设置 FOURCC 失败: {e}")

        # 设置分辨率/FPS（若不支持将保持原值）
        if width:
            self.cap.set(cv2.CAP_PROP_FRAME_WIDTH, width)
        if height:
            self.cap.set(cv2.CAP_PROP_FRAME_HEIGHT, height)
        if fps:
            self.cap.set(cv2.CAP_PROP_FPS, fps)

        actual_w = int(self.cap.get(cv2.CAP_PROP_FRAME_WIDTH))
        actual_h = int(self.cap.get(cv2.CAP_PROP_FRAME_HEIGHT))
        actual_fps = self.cap.get(cv2.CAP_PROP_FPS)
        self.get_logger().info(f"摄像头参数: {actual_w}x{actual_h} @ {actual_fps:.1f}fps")

        # 三级流水线队列
        # 队列1：原始帧 (捕获线程 -> 处理线程)
        self.raw_frame_queue = queue.Queue(maxsize=2)
        # 队列2：处理/编码后的帧 (处理线程 -> 发布线程)
        self.processed_frame_queue = queue.Queue(maxsize=2)
        
        self.capture_thread = None
        self.processing_thread = None  # 新增处理线程
        self.running = True
        
        self.debug_overlay = False  # 默认关闭所有叠加，提供干净视频画面
        
        # 启动两个后台线程
        self.start_capture_thread()
        self.start_processing_thread()  # 新增处理线程
        
        # 定时器现在只负责发布
        self.timer = self.create_timer(self.frame_interval if self.frame_interval > 0 else 0.0, self.publish_frame)

    # def control_callback(self, msg: String):
    #     """(轻量级) 处理控制消息 - 已禁用以降低延迟"""
    #     try:
    #         control_json = json.loads(msg.data)
    #         with self.control_lock:
    #             self.control_data = control_json
    #         # 不要在这里用 info 打印，会刷屏
    #         self.get_logger().debug(f"收到控制消息")
    #     except Exception as e:
    #         self.get_logger().error(f"解析控制消息失败: {e}")

    def start_capture_thread(self):
        """启动独立的捕获线程 (侦察兵)"""
        self.capture_thread = threading.Thread(target=self.capture_loop, daemon=True)
        self.capture_thread.start()
        self.get_logger().info("已启动捕获线程 (侦察兵)")

    def capture_loop(self):
        """(轻量级) 纯粹的捕获循环"""
        while self.running:
            ok, frame = self.cap.read()
            if not ok:
                time.sleep(0.01)
                continue
            try:
                self.raw_frame_queue.put_nowait(frame)
            except queue.Full:
                try:
                    self.raw_frame_queue.get_nowait()  # 丢弃旧帧
                    self.raw_frame_queue.put_nowait(frame)  # 放入新帧
                except queue.Empty:
                    pass

    def start_processing_thread(self):
        """启动独立的处理线程 (翻译官)"""
        self.processing_thread = threading.Thread(target=self.processing_loop, daemon=True)
        self.processing_thread.start()
        self.get_logger().info("已启动处理线程 (翻译官)")

    def processing_loop(self):
        """(CPU密集型) 处理循环 - 干净视频画面，无任何叠加"""
        while self.running:
            try:
                # 阻塞等待新帧，超时1秒
                frame = self.raw_frame_queue.get(timeout=1.0)
                
                h, w = frame.shape[:2]  # 提前获取尺寸
                
                # 仅保留帧计数（无任何绘制操作）
                self.frame_count += 1
                
                # 颜色转换（仅保留必要的转换）
                data = None
                encoding = self.encoding
                step = w
                
                if self.encoding == 'i420':
                    yuv = cv2.cvtColor(frame, cv2.COLOR_BGR2YUV_I420)
                    data = yuv.tobytes()
                    encoding = 'i420'
                elif self.encoding == 'rgb8':
                    rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
                    data = rgb.tobytes()
                    encoding = 'rgb8'
                    step = w * 3
                else:  # bgr8
                    data = frame.tobytes()
                    encoding = 'bgr8'
                    step = w * 3
                
                # 打包数据
                processed_data = (data, h, w, encoding, step)
                
                # 放入已处理队列
                try:
                    self.processed_frame_queue.put_nowait(processed_data)
                except queue.Full:
                    try:
                        self.processed_frame_queue.get_nowait()
                        self.processed_frame_queue.put_nowait(processed_data)
                    except queue.Empty:
                        pass
                        
            except queue.Empty:
                # 队列空，超时，继续循环
                continue
            except Exception as e:
                self.get_logger().error(f"处理线程出错: {e}")

    def publish_frame(self):
        """(轻量级) 发布帧 (发布官)"""
        try:
            # 非阻塞获取处理好的帧
            data, h, w, encoding, step = self.processed_frame_queue.get_nowait()
            
            # 构造消息
            img = Image()
            img.header.stamp = self.get_clock().now().to_msg()
            img.header.frame_id = 'camera_link'
            img.height = h
            img.width = w
            img.encoding = encoding
            img.is_bigendian = 0
            img.step = step
            img.data = data
            
            # 发布！
            self.publisher.publish(img)
            
        except queue.Empty:
            # 没有新帧，跳过
            pass
        except Exception as e:
            self.get_logger().error(f"发布图像失败: {e}")

    def destroy_node(self):
        self.running = False
        if self.cap:
            self.cap.release()
        super().destroy_node()


def main():
    parser = argparse.ArgumentParser(description='LoadExc 摄像头发布器（带控制信息显示）')
    parser.add_argument('--camera-id', type=int, default=0, help='摄像头索引')
    parser.add_argument('--device', type=str, default=None, help='设备路径，优先于 --camera-id，例如 /dev/video1')
    parser.add_argument('--topic', type=str, default='/camera_front_wide', help='ROS2 话题名')
    parser.add_argument('--width', type=int, default=1280, help='图像宽度')
    parser.add_argument('--height', type=int, default=720, help='图像高度')
    parser.add_argument('--fps', type=float, default=15.0, help='目标帧率')
    parser.add_argument('--no-display', action='store_true', help='（已废弃，无头服务器模式）')
    parser.add_argument('--encoding', type=str, default='I420', help='发布图像编码: I420/bgr8/rgb8（默认 I420）')
    parser.add_argument('--fourcc', type=str, default='MJPG', help='摄像头 FOURCC，如 MJPG、YUYV、H264（默认 MJPG 以提升性能）')
    parser.add_argument('--no-debug', action='store_true', help='关闭调试叠加以提升性能')
    args = parser.parse_args()

    camera_source = args.device if args.device else args.camera_id

    rclpy.init()
    node = None
    try:
        node = LoadExcCameraPublisher(
            camera_source=camera_source,
            topic=args.topic,
            width=args.width,
            height=args.height,
            fps=args.fps,
            display=False,  # 强制无头模式
            encoding=args.encoding,
            fourcc=args.fourcc,
        )
        
        # 设置调试叠加
        node.debug_overlay = not args.no_debug
        rclpy.spin(node)
    except KeyboardInterrupt:
        print('\n用户中断，退出')
    finally:
        if node is not None:
            node.destroy_node()
        # 避免重复 shutdown 引发异常
        try:
            rclpy.shutdown()
        except Exception:
            pass


if __name__ == '__main__':
    main()
