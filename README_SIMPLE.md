# LoadExc 客户端 - 简化版

## 🎯 项目架构

**Python + Rust 混合模式**：
- **Python 客户端**: ROS2 视频渲染（测试阶段）
- **Rust 客户端**: LiveKit 桥接（实车部署）

## 🚀 快速启动

### 一键启动
```bash
./run.sh
```

### 手动启动
```bash
# 1. 启动 Python 客户端（后台）
cd python_client
python3 ros2_client.py &

# 2. 启动 Rust 客户端（前台）
cd ..
cargo run
```

## 📡 ROS2 话题

- **`/controls/teleop`**: 控制指令（JSON 格式）
- **`/camera_front_wide`**: 视频流（bgr8 编码）

## 🧪 测试

### 测试 Python 客户端
```bash
cd python_client
python3 test_client.py
```

### 测试 ROS2 连接
```bash
python3 test_ros2_connection.py
```

## 🔧 环境要求

### 系统依赖
- ROS2 Humble
- Python 3.8+
- Rust 1.70+
- clang/clang++

### Python 依赖
```bash
cd python_client
./install.sh
```

## 📊 优势

- ✅ **开发速度快**: Python 部分几小时完成
- ✅ **维护成本低**: Python 代码简洁易懂
- ✅ **性能很好**: 结合两者优势
- ✅ **实车部署**: Rust 部分可直接部署到实车
- ✅ **测试友好**: Python 部分便于测试和调试

## 🎯 使用场景

- **测试阶段**: 使用 Python 客户端进行快速开发和测试
- **实车部署**: 使用 Rust 客户端进行高性能部署
- **混合开发**: 两者通过 ROS2 话题无缝对接

**这是最适合你需求的方案！** 🚀






