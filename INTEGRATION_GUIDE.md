# 🔗 LoadExc 客户端集成指南

## 📋 系统架构

现在你有三种运行模式：

### 1. 🐍 **纯 Python 模式（推荐）**
- **Python 客户端**: 订阅控制消息，发布视频流
- **优点**: 简单、快速、易维护
- **适用**: 大多数场景

### 2. 🦀🐍 **Rust + Python 混合模式**
- **Rust 客户端**: LiveKit 桥接（处理 WebRTC）
- **Python 客户端**: ROS2 视频渲染
- **优点**: 结合两者优势
- **适用**: 需要 LiveKit 功能的场景

### 3. 🦀 **原始 Rust 模式（复杂）**
- **Rust 客户端**: LiveKit 桥接
- **Bevy 仿真器**: 3D 仿真
- **优点**: 完整功能
- **缺点**: 复杂、编译困难

## 🚀 快速开始

### 方法 1: 使用修改后的 run.sh

```bash
# 纯 Python 模式（推荐）
./run.sh python

# Rust + Python 混合模式
./run.sh hybrid

# 原始 Rust 模式
./run.sh legacy

# 测试模式
./run.sh test
```

### 方法 2: 使用新的 run_python.sh

```bash
# 纯 Python 模式
./run_python.sh python

# Rust + Python 混合模式
./run_python.sh hybrid

# 测试模式
./run_python.sh test
```

## 🧪 测试 ROS2 连接

### 1. 测试 Python 客户端
```bash
# 在有 GUI 的环境下运行
cd python_client
python3 test_client.py
```

### 2. 测试 ROS2 话题通信
```bash
# 运行连接测试器
python3 test_ros2_connection.py
```

### 3. 手动测试 ROS2 话题
```bash
# 设置 ROS2 环境
source /opt/ros/humble/setup.bash

# 查看话题列表
ros2 topic list

# 监听控制话题
ros2 topic echo /controls/teleop

# 监听视频话题
ros2 topic echo /camera_front_wide
```

## 🔧 解决 Rust 编译问题

### 问题 1: WebRTC 编译错误
```bash
# 设置 WebRTC 环境变量
export LK_CUSTOM_WEBRTC="$(pwd)/.webrtc/linux-arm64-release"

# 或者使用简化的 ROS2 客户端
cargo run --bin ros2_simple --manifest-path Cargo_ros2.toml
```

### 问题 2: 依赖冲突
```bash
# 清理并重新构建
cargo clean
cargo build
```

## 📡 ROS2 话题配置

### 环境变量
```bash
export ROS_IMAGE_TOPIC="/camera_front_wide"
export ROS_CONTROL_TOPIC="/controls/teleop"
```

### 话题说明
- **`/controls/teleop`** (std_msgs/String): 控制指令 JSON 消息
- **`/camera_front_wide`** (sensor_msgs/Image): 视频流 (bgr8 编码)

## 🎮 控制消息格式

### Gear 消息（档位控制）
```json
{
  "type": "gear",
  "gear": "D",
  "t": 1234567890
}
```

### Analog 消息（模拟控制）
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

## 🔄 集成流程

### 1. 启动 Python 客户端
```bash
# 设置环境
source /opt/ros/humble/setup.bash

# 启动 Python 客户端
cd python_client
python3 ros2_client.py
```

### 2. 启动 Rust 客户端（如果需要）
```bash
# 设置环境
export LK_CUSTOM_WEBRTC="$(pwd)/.webrtc/linux-arm64-release"

# 启动 Rust 客户端
cargo run
```

### 3. 测试通信
```bash
# 运行连接测试
python3 test_ros2_connection.py
```

## 🐛 故障排除

### 常见问题

1. **ROS2 环境未设置**
   ```bash
   source /opt/ros/humble/setup.bash
   ```

2. **Python 依赖缺失**
   ```bash
   cd python_client
   ./install.sh
   ```

3. **Rust 编译失败**
   ```bash
   # 使用简化版本
   cargo run --bin ros2_simple --manifest-path Cargo_ros2.toml
   ```

4. **话题连接失败**
   ```bash
   # 检查话题
   ros2 topic list
   ros2 topic info /controls/teleop
   ```

## 📊 性能对比

| 模式 | 开发时间 | 维护难度 | 性能 | 推荐度 |
|------|----------|----------|------|--------|
| **Python** | ✅ 几小时 | ✅ 极低 | ✅ 足够 | ⭐⭐⭐⭐⭐ |
| **Rust+Python** | ⚠️ 中等 | ⚠️ 中等 | ✅ 很好 | ⭐⭐⭐⭐ |
| **Rust+Bevy** | ❌ 几天 | ❌ 极高 | ✅ 最好 | ⭐⭐ |

## 🎯 推荐方案

**对于你的需求，强烈推荐使用纯 Python 模式：**

1. ✅ **开发速度快**: 几小时完成
2. ✅ **维护成本低**: 代码简洁易懂
3. ✅ **性能足够**: 30 FPS 稳定运行
4. ✅ **生态丰富**: Python 库支持完善
5. ✅ **调试方便**: 实时预览和测试

**你的"姐姐"说得对：为了拧一颗螺丝，不需要去挖矿炼钢！直接用螺丝刀就行了！** 🔧✨






