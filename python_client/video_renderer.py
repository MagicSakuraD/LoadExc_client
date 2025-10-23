"""
视频渲染器 - 在黑色背景上显示控制指令
"""
import cv2
import numpy as np
import time
from typing import Tuple
from control_message import UnifiedControlMessage


class VideoRenderer:
    """视频渲染器类"""
    
    def __init__(self, width: int = 1280, height: int = 720):
        self.width = width
        self.height = height
        self.font = cv2.FONT_HERSHEY_SIMPLEX
        self.font_scale = 1.0
        self.font_thickness = 2
        
        # 颜色定义 (BGR格式)
        self.colors = {
            'yellow': (0, 255, 255),
            'orange': (0, 165, 255),
            'green': (0, 255, 0),
            'red': (0, 0, 255),
            'blue': (255, 0, 0),
            'white': (255, 255, 255),
            'cyan': (255, 255, 0)
        }
    
    def render_frame(self, control_data: UnifiedControlMessage) -> np.ndarray:
        """渲染一帧视频"""
        # 创建黑色背景
        frame = np.zeros((self.height, self.width, 3), dtype=np.uint8)
        
        # 计算延迟
        now_ms = int(time.time() * 1000)
        latency_ms = now_ms - control_data.timestamp
        
        # 绘制时间戳
        self._draw_text(frame, f"Time: {now_ms} ms", (30, 50), self.colors['yellow'])
        
        # 绘制延迟
        latency_color = self.colors['green'] if latency_ms < 100 else self.colors['orange'] if latency_ms < 500 else self.colors['red']
        self._draw_text(frame, f"Latency: {latency_ms} ms", (30, 100), latency_color)
        
        # 绘制控制数据
        y_start = 150
        line_height = 30
        
        # 标题
        self._draw_text(frame, "Control Data:", (30, y_start), self.colors['cyan'], scale=1.2)
        y_start += 50
        
        # 基础控制
        self._draw_text(frame, f"Gear: {control_data.gear}", (30, y_start), self.colors['white'])
        y_start += line_height
        
        self._draw_text(frame, f"Throttle: {control_data.throttle:.2f}", (30, y_start), self.colors['green'])
        y_start += line_height
        
        self._draw_text(frame, f"Brake: {control_data.brake:.2f}", (30, y_start), self.colors['red'])
        y_start += line_height
        
        self._draw_text(frame, f"Rotation: {control_data.rotation:.2f}", (30, y_start), self.colors['blue'])
        y_start += line_height
        
        # 装载机专用控制
        y_start += 20
        self._draw_text(frame, "Loader Controls:", (30, y_start), self.colors['cyan'], scale=1.2)
        y_start += 50
        
        self._draw_text(frame, f"Boom: {control_data.boom:.2f}", (30, y_start), self.colors['white'])
        y_start += line_height
        
        self._draw_text(frame, f"Bucket: {control_data.bucket:.2f}", (30, y_start), self.colors['white'])
        y_start += line_height
        
        # 兼容性控制（可选显示）
        if any([control_data.left_track, control_data.right_track, control_data.swing, control_data.stick]):
            y_start += 20
            self._draw_text(frame, "Additional Controls:", (30, y_start), self.colors['cyan'], scale=1.2)
            y_start += 50
            
            if control_data.left_track != 0:
                self._draw_text(frame, f"Left Track: {control_data.left_track:.2f}", (30, y_start), self.colors['white'])
                y_start += line_height
            
            if control_data.right_track != 0:
                self._draw_text(frame, f"Right Track: {control_data.right_track:.2f}", (30, y_start), self.colors['white'])
                y_start += line_height
            
            if control_data.swing != 0:
                self._draw_text(frame, f"Swing: {control_data.swing:.2f}", (30, y_start), self.colors['white'])
                y_start += line_height
            
            if control_data.stick != 0:
                self._draw_text(frame, f"Stick: {control_data.stick:.2f}", (30, y_start), self.colors['white'])
                y_start += line_height
        
        # 绘制 JSON 数据（调试用）
        y_start += 20
        self._draw_text(frame, "JSON Data:", (30, y_start), self.colors['cyan'], scale=1.2)
        y_start += 50
        
        json_text = control_data.to_json()
        for i, line in enumerate(json_text.split('\n')):
            if i > 10:  # 限制显示行数
                self._draw_text(frame, "...", (30, y_start + i * 20), self.colors['white'], scale=0.5)
                break
            self._draw_text(frame, line, (30, y_start + i * 20), self.colors['white'], scale=0.5)
        
        return frame
    
    def render_frame_for_streaming(self, control_data: UnifiedControlMessage) -> np.ndarray:
        """渲染用于推流的视频帧（包含完整遥测数据）"""
        # 创建黑色背景
        frame = np.zeros((self.height, self.width, 3), dtype=np.uint8)
        
        # 计算延迟
        now_ms = int(time.time() * 1000)
        latency_ms = now_ms - control_data.timestamp
        
        # 绘制时间戳
        self._draw_text(frame, f"Time: {now_ms} ms", (30, 50), self.colors['yellow'])
        
        # 绘制延迟
        latency_color = self.colors['green'] if latency_ms < 100 else self.colors['orange'] if latency_ms < 500 else self.colors['red']
        self._draw_text(frame, f"Latency: {latency_ms} ms", (30, 100), latency_color)
        
        # 绘制控制数据
        y_start = 150  # 从150开始，包含延迟行
        line_height = 30
        
        # 标题
        self._draw_text(frame, "Control Data:", (30, y_start), self.colors['cyan'], scale=1.2)
        y_start += 50
        
        # 基础控制
        self._draw_text(frame, f"Gear: {control_data.gear}", (30, y_start), self.colors['white'])
        y_start += line_height
        
        self._draw_text(frame, f"Throttle: {control_data.throttle:.2f}", (30, y_start), self.colors['green'])
        y_start += line_height
        
        self._draw_text(frame, f"Brake: {control_data.brake:.2f}", (30, y_start), self.colors['red'])
        y_start += line_height
        
        self._draw_text(frame, f"Rotation: {control_data.rotation:.2f}", (30, y_start), self.colors['blue'])
        y_start += line_height
        
        # 装载机专用控制
        y_start += 20
        self._draw_text(frame, "Loader Controls:", (30, y_start), self.colors['cyan'], scale=1.2)
        y_start += 50
        
        self._draw_text(frame, f"Boom: {control_data.boom:.2f}", (30, y_start), self.colors['white'])
        y_start += line_height
        
        self._draw_text(frame, f"Bucket: {control_data.bucket:.2f}", (30, y_start), self.colors['white'])
        y_start += line_height
        
        # 兼容性控制（可选显示）
        if any([control_data.left_track, control_data.right_track, control_data.swing, control_data.stick]):
            y_start += 20
            self._draw_text(frame, "Additional Controls:", (30, y_start), self.colors['cyan'], scale=1.2)
            y_start += 50
            
            if control_data.left_track != 0:
                self._draw_text(frame, f"Left Track: {control_data.left_track:.2f}", (30, y_start), self.colors['white'])
                y_start += line_height
            
            if control_data.right_track != 0:
                self._draw_text(frame, f"Right Track: {control_data.right_track:.2f}", (30, y_start), self.colors['white'])
                y_start += line_height
            
            if control_data.swing != 0:
                self._draw_text(frame, f"Swing: {control_data.swing:.2f}", (30, y_start), self.colors['white'])
                y_start += line_height
            
            if control_data.stick != 0:
                self._draw_text(frame, f"Stick: {control_data.stick:.2f}", (30, y_start), self.colors['white'])
                y_start += line_height
        
        # 绘制 JSON 数据（调试用）
        y_start += 20
        self._draw_text(frame, "JSON Data:", (30, y_start), self.colors['cyan'], scale=1.2)
        y_start += 50
        
        json_text = control_data.to_json()
        for i, line in enumerate(json_text.split('\n')):
            if i > 10:  # 限制显示行数
                self._draw_text(frame, "...", (30, y_start + i * 20), self.colors['white'], scale=0.5)
                break
            self._draw_text(frame, line, (30, y_start + i * 20), self.colors['white'], scale=0.5)
        
        return frame
    
    def _draw_text(self, frame: np.ndarray, text: str, position: Tuple[int, int], 
                   color: Tuple[int, int, int], scale: float = None, thickness: int = None):
        """绘制文本的辅助方法"""
        scale = scale or self.font_scale
        thickness = thickness or self.font_thickness
        
        cv2.putText(frame, text, position, self.font, scale, color, thickness)
    
    def create_test_frame(self) -> np.ndarray:
        """创建测试帧（用于调试）"""
        frame = np.zeros((self.height, self.width, 3), dtype=np.uint8)
        
        # 绘制测试信息
        self._draw_text(frame, "Python Video Renderer - Test Frame", (30, 50), self.colors['yellow'], scale=1.5)
        self._draw_text(frame, f"Resolution: {self.width}x{self.height}", (30, 100), self.colors['green'])
        self._draw_text(frame, "Status: Ready", (30, 150), self.colors['green'])
        
        return frame
