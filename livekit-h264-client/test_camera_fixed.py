#!/usr/bin/env python3
"""
基于官方示例修复的摄像头测试 - 使用正确的LiveKit API
"""
import asyncio
import logging
import time
import os
import sys
from pathlib import Path
import cv2
import numpy as np
from signal import SIGINT, SIGTERM
from time import perf_counter

# 添加src目录到Python路径
sys.path.insert(0, str(Path(__file__).parent / "src"))

from livekit import rtc
from scripts.get_token import get_livekit_token

# 设置日志
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

WIDTH, HEIGHT = 1280, 720
FPS = 30

class CameraCapture:
    def __init__(self, device="/dev/video0"):
        self.device = device
        self.cap = None
        
    def start(self):
        """启动摄像头"""
        self.cap = cv2.VideoCapture(self.device)
        if not self.cap.isOpened():
            # 尝试不同的后端
            self.cap = cv2.VideoCapture(self.device, cv2.CAP_V4L2)
            if not self.cap.isOpened():
                raise RuntimeError(f"无法打开摄像头 {self.device}")
        
        # 设置摄像头参数
        self.cap.set(cv2.CAP_PROP_FRAME_WIDTH, WIDTH)
        self.cap.set(cv2.CAP_PROP_FRAME_HEIGHT, HEIGHT)
        self.cap.set(cv2.CAP_PROP_FPS, FPS)
        
        logger.info(f"摄像头已启动: {self.device}")
        
    def read_frame(self):
        """读取一帧"""
        ret, frame = self.cap.read()
        if ret:
            # 调整大小
            frame = cv2.resize(frame, (WIDTH, HEIGHT))
            # 转换BGR到RGBA
            frame_rgba = cv2.cvtColor(frame, cv2.COLOR_BGR2RGBA)
            return frame_rgba
        return None
        
    def stop(self):
        """停止摄像头"""
        if self.cap:
            self.cap.release()
            logger.info("摄像头已停止")

async def camera_stream_loop(source: rtc.VideoSource, camera: CameraCapture):
    """摄像头流循环 - 基于官方示例的帧率控制"""
    framerate = 1 / FPS
    next_frame_time = perf_counter()
    frame_count = 0
    start_time = time.time()
    
    while True:
        try:
            frame_data = camera.read_frame()
            if frame_data is not None:
                # 创建VideoFrame - 使用正确的格式
                frame = rtc.VideoFrame(WIDTH, HEIGHT, rtc.VideoBufferType.RGBA, frame_data.tobytes())
                source.capture_frame(frame)
                
                # 性能监控
                frame_count += 1
                if frame_count % 100 == 0:
                    elapsed = time.time() - start_time
                    fps = frame_count / elapsed
                    logger.info(f"推流性能: {fps:.1f} fps, 帧数: {frame_count}")
            else:
                logger.warning("无法读取摄像头帧")
                
        except Exception as e:
            logger.error(f"处理帧失败: {e}")
            
        # 精确的帧率控制 - 基于官方示例
        next_frame_time += 1 / FPS
        await asyncio.sleep(max(0, next_frame_time - perf_counter()))

async def main():
    """主函数 - 基于官方示例结构"""
    try:
        # 自动获取token
        token = get_livekit_token()
        if not token:
            logger.error("无法获取token")
            return
        
        livekit_url = "ws://111.186.56.118:7880"
        logger.info(f"连接到LiveKit: {livekit_url}")
        
        # 创建房间
        room = rtc.Room()
        
        # 设置事件处理器
        @room.on("participant_connected")
        def on_participant_connected(participant: rtc.RemoteParticipant):
            logger.info(f"参与者连接: {participant.identity}")
        
        @room.on("participant_disconnected")
        def on_participant_disconnected(participant: rtc.RemoteParticipant):
            logger.info(f"参与者断开: {participant.identity}")
        
        # 连接到房间
        await room.connect(livekit_url, token)
        logger.info(f"已连接到LiveKit房间: {room.name}")
        
        # 创建视频源和轨道 - 使用官方示例的方式
        source = rtc.VideoSource(WIDTH, HEIGHT)
        track = rtc.LocalVideoTrack.create_video_track("camera", source)
        
        # 发布视频轨道 - 使用官方示例的选项
        options = rtc.TrackPublishOptions(
            source=rtc.TrackSource.SOURCE_CAMERA,
            simulcast=True,
            video_encoding=rtc.VideoEncoding(
                max_framerate=FPS,
                max_bitrate=3_000_000,
            ),
        )
        
        publication = await room.local_participant.publish_track(track, options)
        logger.info(f"视频轨道已发布: {publication.sid}")
        
        # 启动摄像头
        device = os.getenv("CAMERA_DEVICE", "/dev/video0")
        camera = CameraCapture(device)
        camera.start()
        
        logger.info("开始推流摄像头画面...")
        
        # 启动摄像头流循环
        await camera_stream_loop(source, camera)
        
    except KeyboardInterrupt:
        logger.info("收到中断信号，停止推流...")
    except Exception as e:
        logger.error(f"运行失败: {e}")
    finally:
        # 清理资源
        try:
            if 'camera' in locals():
                camera.stop()
        except:
            pass
        try:
            if 'room' in locals():
                await room.disconnect()
        except:
            pass
        logger.info("已断开连接")

if __name__ == "__main__":
    # 设置信号处理 - 基于官方示例
    loop = asyncio.get_event_loop()
    
    async def cleanup():
        logger.info("正在清理资源...")
        loop.stop()
    
    for signal in [SIGINT, SIGTERM]:
        loop.add_signal_handler(signal, lambda: asyncio.ensure_future(cleanup()))
    
    try:
        asyncio.ensure_future(main())
        loop.run_forever()
    finally:
        loop.close()

