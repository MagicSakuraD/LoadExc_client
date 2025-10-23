#!/bin/bash

# LoadExc 摄像头发布器依赖安装脚本

set -euo pipefail

echo "🔧 安装 LoadExc 摄像头发布器依赖..."

# 检查系统
echo "📋 系统信息:"
echo "   OS: $(lsb_release -d 2>/dev/null | cut -f2 || echo 'Unknown')"
echo "   Python: $(python3 --version)"
echo "   OpenCV: $(python3 -c 'import cv2; print(cv2.__version__)' 2>/dev/null || echo 'Not installed')"

# 安装系统依赖
echo "📦 安装系统依赖..."
sudo apt update
sudo apt install -y \
    python3-pip \
    python3-opencv \
    python3-numpy \
    libopencv-dev \
    libgtk-3-dev \
    libglib2.0-dev \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev

# 安装ROS2依赖
echo "🤖 安装ROS2依赖..."
if [ -f /opt/ros/humble/setup.bash ]; then
    source /opt/ros/humble/setup.bash
    sudo apt install -y \
        ros-humble-cv-bridge \
        ros-humble-sensor-msgs \
        ros-humble-image-transport
else
    echo "⚠️  未找到ROS2 Humble，请先安装ROS2"
fi

# 安装Python依赖
echo "🐍 安装Python依赖..."
pip3 install --upgrade pip
pip3 install -r requirements.txt

# 检查摄像头权限
echo "📷 检查摄像头权限..."
if ! groups | grep -q video; then
    echo "⚠️  用户不在video组，正在添加..."
    sudo usermod -a -G video $USER
    echo "✅ 已添加到video组，请重新登录或重启"
fi

# 检查摄像头设备
echo "📹 检查摄像头设备..."
if ls /dev/video* >/dev/null 2>&1; then
    echo "✅ 找到摄像头设备:"
    ls -l /dev/video*
else
    echo "❌ 未找到摄像头设备"
fi

echo "✅ 依赖安装完成！"
echo ""
echo "🚀 使用方法:"
echo "   python3 camera_publisher.py                    # 启动摄像头发布器"
echo "   python3 camera_publisher.py --no-display       # 无显示模式"
echo "   python3 camera_publisher.py --encoding I420     # I420格式发布"
echo ""
echo "🔧 如果仍有GTK问题，可以尝试:"
echo "   export QT_QPA_PLATFORM=offscreen"
echo "   python3 camera_publisher.py --no-display"



