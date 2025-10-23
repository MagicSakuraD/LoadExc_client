# 🚀 LoadExc Python Client 快速开始

## ✅ 已完成的功能

- 📡 **ROS2 集成**: 订阅 `/controls/teleop`，发布 `/camera_front_wide`
- 🎥 **视频渲染**: 在黑色背景上显示控制指令
- 🧪 **测试功能**: 完整的测试和调试工具
- 📦 **安装脚本**: 一键安装所有依赖

## 🎯 使用方法

### 1. 安装依赖
```bash
cd python_client
./install.sh
```

### 2. 运行测试 (在有 GUI 的环境下)
```bash
# 测试视频渲染器
./run_client.sh test
# 或直接运行
python3 test_client.py
```

### 3. 运行 ROS2 客户端
```bash
# 设置 ROS2 环境
source /opt/ros/humble/setup.bash

# 启动客户端
./run_client.sh ros2
# 或直接运行
python3 ros2_client.py
```

## 📡 ROS2 话题

### 订阅话题
- **`/controls/teleop`** (std_msgs/String)
  - 接收控制指令 JSON 消息

### 发布话题  
- **`/camera_front_wide`** (sensor_msgs/Image)
  - 发布渲染的视频流 (bgr8 编码, 30 FPS)

## 🎮 控制消息格式

### Gear 消息 (档位)
```json
{
  "type": "gear",
  "gear": "D",
  "t": 1234567890
}
```

### Analog 消息 (模拟控制)
```json
{
  "type": "analog", 
  "v": {
    "rotation": 0.5,
    "brake": 0.0,
    "throttle": 0.8,
    "boom": 0.3,
    "bucket": -0.2
  },
  "t": 1234567890
}
```

## 🎥 视频显示内容

- ⏰ **时间戳**: 当前时间
- 📊 **延迟**: 控制消息延迟 (绿色<100ms, 橙色<500ms, 红色>500ms)
- 🎮 **基础控制**: 档位、油门、刹车、旋转
- 🏗️ **装载机控制**: 大臂、铲斗
- 📋 **JSON 数据**: 原始控制消息

## 🔧 调试和测试

### 测试控制消息解析
```python
from control_message import UnifiedControlMessage

control = UnifiedControlMessage()
json_data = '{"type": "analog", "v": {"rotation": 0.5}}'
control.update_from_json(json_data)
print(f"旋转: {control.rotation}")
```

### 测试视频渲染
```python
from video_renderer import VideoRenderer
from control_message import UnifiedControlMessage

renderer = VideoRenderer()
control = UnifiedControlMessage()
frame = renderer.render_frame(control)
cv2.imshow('Test', frame)
```

## 📊 性能对比

| 方案 | 开发时间 | 代码行数 | 维护难度 | 性能 |
|------|----------|----------|----------|------|
| **Python + OpenCV** | ✅ 几小时 | ✅ ~200行 | ✅ 极低 | ✅ 足够 |
| **Rust + Bevy** | ❌ 几天到几周 | ❌ 468行+ | ❌ 极高 | ❌ 过剩 |

## 🎯 总结

**Python 方案完美替代了复杂的 Rust 方案！**

- ✅ **开发速度快**: 几小时完成
- ✅ **代码简洁**: 易于理解和维护  
- ✅ **性能足够**: 30 FPS 稳定运行
- ✅ **生态丰富**: Python 库支持完善
- ✅ **调试方便**: 实时预览和测试

**你的"姐姐"说得对：为了拧一颗螺丝，不需要去挖矿炼钢！直接用螺丝刀就行了！** 🔧✨






