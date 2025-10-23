"""
统一控制消息结构定义
与 Rust 版本保持完全一致
"""
import time
import json
from typing import Dict, Any


class UnifiedControlMessage:
    """统一控制消息结构（合并 gear 和 analog）"""
    
    def __init__(self):
        # 装载机专用控制
        self.rotation: float = 0.0      # 方向盘旋转: -1 (左) to 1 (右)
        self.brake: float = 0.0         # 刹车: 0 (松开) to 1 (踩死)
        self.throttle: float = 0.0      # 油门: 0 (松开) to 1 (踩死)
        self.gear: str = "N"            # 档位: 'P' | 'R' | 'N' | 'D'
        
        # 共用控制
        self.boom: float = 0.0         # 大臂: -1 (降) to 1 (提)
        self.bucket: float = 0.0        # 铲斗: -1 (收) to 1 (翻)
        
        # 兼容性属性（设为默认值）
        self.left_track: float = 0.0    # 左履带: -1 (后) to 1 (前)
        self.right_track: float = 0.0   # 右履带: -1 (后) to 1 (前)
        self.swing: float = 0.0         # 驾驶室旋转: -1 (左) to 1 (右)
        self.stick: float = 0.0        # 小臂: -1 (收) to 1 (伸)
        
        # 设备类型标识
        self.device_type: str = "wheel_loader"
        self.timestamp: int = int(time.time() * 1000)
    
    def update_from_json(self, json_data: str) -> bool:
        """从 JSON 字符串更新控制消息"""
        try:
            data = json.loads(json_data)
            
            # 更新时间戳
            if 't' in data:
                self.timestamp = data['t']
            
            # 根据消息类型更新相应字段
            if 'type' in data:
                msg_type = data['type']
                if msg_type == 'gear' and 'gear' in data:
                    self.gear = data['gear']
                elif msg_type == 'analog' and 'v' in data:
                    v_obj = data['v']
                    self.rotation = v_obj.get('rotation', self.rotation)
                    self.brake = v_obj.get('brake', self.brake)
                    self.throttle = v_obj.get('throttle', self.throttle)
                    self.boom = v_obj.get('boom', self.boom)
                    self.bucket = v_obj.get('bucket', self.bucket)
                    self.left_track = v_obj.get('leftTrack', self.left_track)
                    self.right_track = v_obj.get('rightTrack', self.right_track)
                    self.swing = v_obj.get('swing', self.swing)
                    self.stick = v_obj.get('stick', self.stick)
            
            return True
        except (json.JSONDecodeError, KeyError, TypeError) as e:
            print(f"⚠️ 解析控制消息失败: {e}")
            return False
    
    def to_json(self) -> str:
        """转换为 JSON 字符串"""
        data = {
            'rotation': self.rotation,
            'brake': self.brake,
            'throttle': self.throttle,
            'gear': self.gear,
            'boom': self.boom,
            'bucket': self.bucket,
            'left_track': self.left_track,
            'right_track': self.right_track,
            'swing': self.swing,
            'stick': self.stick,
            'device_type': self.device_type,
            'timestamp': self.timestamp
        }
        return json.dumps(data, indent=2)
    
    def to_dict(self) -> Dict[str, Any]:
        """转换为字典"""
        return {
            'rotation': self.rotation,
            'brake': self.brake,
            'throttle': self.throttle,
            'gear': self.gear,
            'boom': self.boom,
            'bucket': self.bucket,
            'left_track': self.left_track,
            'right_track': self.right_track,
            'swing': self.swing,
            'stick': self.stick,
            'device_type': self.device_type,
            'timestamp': self.timestamp
        }
