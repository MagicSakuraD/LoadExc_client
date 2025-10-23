#!/bin/bash

# LoadExc 客户端启动脚本
# Python + Rust 混合模式：
# - Python 客户端: ROS2 视频渲染
# - Rust 客户端: LiveKit 桥接

set -euo pipefail

echo "🚀 启动 LoadExc_client 系统..."
echo "📋 使用说明:"
echo "   - 无头服务器模式（性能更好）"
echo "   - 摄像头视频流 + 控制信息叠加"
echo "   - 发布到 /camera_front_wide 话题"
echo ""

# 设置 ROS2 环境
if [ -f /opt/ros/humble/setup.sh ]; then
  echo "🔧 设置 ROS2 环境..."
  set +u
  . /opt/ros/humble/setup.sh
  set -u
  echo "✅ ROS2 环境已设置: $ROS_DISTRO"
fi

if [ -f "$HOME/rust_ws/install/setup.sh" ]; then
  echo "🔧 设置 ROS2 工作区环境..."
  set +u
  . "$HOME/rust_ws/install/setup.sh"
  set -u
fi

# 设置 ROS2 话题
export ROS_IMAGE_TOPIC="${ROS_IMAGE_TOPIC:-/camera_front_wide}"
export ROS_CONTROL_TOPIC="${ROS_CONTROL_TOPIC:-/controls/teleop}"

echo "📡 ROS2 话题配置:"
echo "   图像话题: $ROS_IMAGE_TOPIC"
echo "   控制话题: $ROS_CONTROL_TOPIC"

echo "🦀🐍 启动 Python + Rust 混合模式..."
echo "   - Python 客户端: ROS2 视频渲染"
echo "   - Rust 客户端: LiveKit 桥接"

# 设置 WebRTC 环境变量
if [ -z "${LK_CUSTOM_WEBRTC:-}" ]; then
    export LK_CUSTOM_WEBRTC="$(pwd)/.webrtc/linux-arm64-release"
fi

# 设置编译器为 clang（解决 WebRTC 编译问题）
export CC=clang
export CXX=clang++

# 检查 Rust 编译
echo "🛠️  构建 Rust 客户端..."
if ! cargo check 2>/dev/null; then
    echo "⚠️  Rust 编译有问题，尝试修复..."
    # 这里可以添加修复逻辑
fi

# 检查 Python 依赖
if ! python3 -c "import cv2, numpy, rclpy" 2>/dev/null; then
    echo "📦 安装 Python 依赖..."
    cd python_client
    ./install.sh
    cd ..
fi

# 启动 Python 客户端（后台）
echo "🐍 启动 LoadExc 摄像头发布器（无头服务器模式）..."
cd python_client

# 选择摄像头发布器版本
if [ "${MINIMAL_MODE:-0}" = "1" ]; then
    echo "⚡ 使用最小延迟模式..."
    python3 camera_publisher_minimal.py --encoding I420 --fps 30 &
else
    echo "📊 使用标准模式（干净视频画面）..."
    python3 camera_publisher.py --encoding I420 --no-debug &
fi

PYTHON_PID=$!
cd ..

# 等待 Python 客户端启动
sleep 2

# 启动 Rust 客户端（前台）
echo "🦀 启动 Rust LiveKit 桥接..."
cargo run "$@"

# 清理
trap "kill $PYTHON_PID 2>/dev/null || true" EXIT


