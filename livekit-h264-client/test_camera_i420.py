#!/usr/bin/env python3
"""
正确的I420格式摄像头推流实现
基于LiveKit官方API，避免H.264编码/解码的额外延迟
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

# 添加src目录到Python路径
sys.path.insert(0, str(Path(__file__).parent / "src"))

from livekit import rtc
from scripts.get_token import get_livekit_token

# 设置日志
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

WIDTH, HEIGHT = 1280, 720
FPS = 30

class I420CameraCapture:
    def __init__(self, device="/dev/video0"):
        self.device = device
        self.cap = None
        self.actual_format = None
        
    def start(self):
        """启动摄像头，优先使用YUYV格式"""
        self.cap = cv2.VideoCapture(self.device, cv2.CAP_V4L2)
        if not self.cap.isOpened():
            raise RuntimeError(f"无法打开摄像头 {self.device}")
        
        # 尝试设置YUYV格式（硬件原生YUV格式）
        self.cap.set(cv2.CAP_PROP_FOURCC, cv2.VideoWriter_fourcc(*'YUYV'))
        self.cap.set(cv2.CAP_PROP_FRAME_WIDTH, WIDTH)
        self.cap.set(cv2.CAP_PROP_FRAME_HEIGHT, HEIGHT)
        self.cap.set(cv2.CAP_PROP_FPS, FPS)
        
        # 检查实际格式
        actual_fourcc = int(self.cap.get(cv2.CAP_PROP_FOURCC))
        self.actual_format = "".join([chr((actual_fourcc >> 8 * i) & 0xFF) for i in range(4)])
        logger.info(f"摄像头已启动: {self.device}, 格式: {self.actual_format}")
        
    def read_frame(self):
        """读取一帧并转换为I420格式"""
        ret, frame = self.cap.read()
        if not ret:
            return None
        
        # 根据实际格式选择最优转换方式
        if self.actual_format == 'YUYV':
            # YUYV格式，尝试直接转换
            try:
                # 注意：OpenCV的read()可能已经自动转换为BGR
                # 如果frame确实是YUYV格式，使用YUYV2YUV_I420
                if len(frame.shape) == 2:  # YUYV通常是2D数组
                    yuv = cv2.cvtColor(frame, cv2.COLOR_YUYV2YUV_I420)
                else:
                    # OpenCV已经转换为BGR，使用BGR转换
                    yuv = cv2.cvtColor(frame, cv2.COLOR_BGR2YUV_I420)
            except cv2.error:
                # 兜底：如果YUYV转换失败，当作BGR处理
                logger.warning("YUYV转换失败，使用BGR转换")
                yuv = cv2.cvtColor(frame, cv2.COLOR_BGR2YUV_I420)
        else:
            # 其他格式（MJPG等），通常已经是BGR
            yuv = cv2.cvtColor(frame, cv2.COLOR_BGR2YUV_I420)
        
        return yuv
    
        
    def stop(self):
        """停止摄像头"""
        if self.cap:
            self.cap.release()

# 直接使用 rtc.VideoSource，不需要包装类

async def main():
    """主函数"""
    # 获取环境变量
    livekit_url = os.getenv("LIVEKIT_URL", "ws://111.186.56.118:7880")
    camera_device = os.getenv("CAMERA_DEVICE", "/dev/video0")
    
    logger.info(f"连接到LiveKit: {livekit_url}")
    
    # 获取token
    try:
        token = get_livekit_token()
        logger.info("Token获取成功")
    except Exception as e:
        logger.error(f"Token获取失败: {e}")
        return
    
    # 创建LiveKit房间
    room = rtc.Room()
    
    # 设置事件处理器
    @room.on("participant_connected")
    def on_participant_connected(participant: rtc.RemoteParticipant):
        logger.info(f"参与者连接: {participant.identity}")
    
    @room.on("participant_disconnected")
    def on_participant_disconnected(participant: rtc.RemoteParticipant):
        logger.info(f"参与者断开: {participant.identity}")
    
    @room.on("local_track_published")
    def on_local_track_published(publication: rtc.LocalTrackPublication, track):
        logger.info(f"本地轨道已发布: {publication.sid}, 类型: {publication.kind}")
    
    @room.on("track_published")
    def on_track_published(publication: rtc.RemoteTrackPublication, participant: rtc.RemoteParticipant):
        logger.info(f"远程轨道已发布: {publication.sid}, 参与者: {participant.identity}")
    
    @room.on("track_subscribed")
    def on_track_subscribed(track: rtc.Track, publication: rtc.RemoteTrackPublication, participant: rtc.RemoteParticipant):
        logger.info(f"轨道已订阅: {publication.sid}, 参与者: {participant.identity}")
    
    # 连接到房间
    await room.connect(livekit_url, token)
    logger.info(f"已连接到LiveKit房间: {room.name}")
    
    # 打印当前参与者信息
    logger.info(f"当前参与者数量: {len(room.remote_participants)}")
    for participant in room.remote_participants.values():
        logger.info(f"远程参与者: {participant.identity}")
        for track in participant.track_publications.values():
            logger.info(f"  - 轨道: {track.sid}, 类型: {track.kind}")
    
    # 创建视频源
    video_source = rtc.VideoSource(WIDTH, HEIGHT)
    
    # 发布视频轨道 - 添加发布选项
    video_track = rtc.LocalVideoTrack.create_video_track("camera", video_source)
    
    # 发布选项 - 与test_camera_fixed.py保持一致
    options = rtc.TrackPublishOptions(
        source=rtc.TrackSource.SOURCE_CAMERA,
        simulcast=True,
        video_encoding=rtc.VideoEncoding(
            max_framerate=FPS,
            max_bitrate=3_000_000,
        ),
    )
    
    publication = await room.local_participant.publish_track(video_track, options)
    logger.info(f"视频轨道已发布: {video_track.sid}")
    logger.info(f"发布信息: {publication.sid}, 类型: {publication.kind}")
    
    # 等待一下让轨道完全发布
    await asyncio.sleep(1)
    
    # 检查本地参与者的轨道
    logger.info(f"本地参与者轨道数量: {len(room.local_participant.track_publications)}")
    for track in room.local_participant.track_publications.values():
        logger.info(f"本地轨道: {track.sid}, 类型: {track.kind}")
    
    # 启动摄像头
    camera = I420CameraCapture(camera_device)
    camera.start()
    
    logger.info("开始推流摄像头画面...")
    
    # 正确的帧率控制
    frame_duration = 1.0 / FPS
    loop = asyncio.get_event_loop()
    
    try:
        while True:
            loop_start_time = time.monotonic()
            
            # 异步执行阻塞的read_frame
            i420_data = await loop.run_in_executor(None, camera.read_frame)
            
            if i420_data is not None:
                # 创建VideoFrame并推送到LiveKit
                frame = rtc.VideoFrame(
                    WIDTH, HEIGHT, 
                    rtc.VideoBufferType.I420, 
                    i420_data.tobytes()
                )
                video_source.capture_frame(frame)
            else:
                logger.warning("无法读取摄像头帧")
            
            # 计算处理时间并调整睡眠时间
            process_time = time.monotonic() - loop_start_time
            sleep_time = frame_duration - process_time
            
            if sleep_time > 0:
                await asyncio.sleep(sleep_time)
            # else: 帧处理超时，立即开始下一帧
                
    except KeyboardInterrupt:
        logger.info("收到中断信号，正在停止...")
    except Exception as e:
        logger.error(f"推流过程中出错: {e}")
    finally:
        # 清理资源
        camera.stop()
        await room.disconnect()
        logger.info("已断开连接")

if __name__ == "__main__":
    # 设置信号处理
    def signal_handler(signum, frame):
        logger.info(f"收到信号 {signum}，正在退出...")
        sys.exit(0)
    
    import signal
    signal.signal(SIGINT, signal_handler)
    signal.signal(SIGTERM, signal_handler)
    
    # 运行主程序
    asyncio.run(main())
