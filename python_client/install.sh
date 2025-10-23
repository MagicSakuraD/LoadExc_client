#!/bin/bash

# LoadExc Python Client 安装脚本

echo "🚀 安装 LoadExc Python Client..."

# 检查 Python 版本
python3 --version
if [ $? -ne 0 ]; then
    echo "❌ 错误: 未找到 Python3"
    exit 1
fi

# 检查 pip
pip3 --version
if [ $? -ne 0 ]; then
    echo "❌ 错误: 未找到 pip3"
    exit 1
fi

# 安装 Python 依赖
echo "📦 安装 Python 依赖..."
pip3 install -r requirements.txt

if [ $? -ne 0 ]; then
    echo "❌ 错误: Python 依赖安装失败"
    exit 1
fi

# 检查 ROS2 环境
echo "🔍 检查 ROS2 环境..."
if [ -z "$ROS_DISTRO" ]; then
    echo "⚠️  警告: ROS2 环境未设置"
    echo "   请运行: source /opt/ros/humble/setup.bash"
else
    echo "✅ ROS2 环境已设置: $ROS_DISTRO"
fi

# 检查 cv_bridge
echo "🔍 检查 cv_bridge..."
python3 -c "import cv_bridge" 2>/dev/null
if [ $? -ne 0 ]; then
    echo "📦 安装 cv_bridge..."
    pip3 install cv-bridge
fi

echo "✅ 安装完成!"
echo ""
echo "📋 使用方法:"
echo "  1. 设置 ROS2 环境: source /opt/ros/humble/setup.bash"
echo "  2. 运行测试: python3 test_client.py"
echo "  3. 运行客户端: python3 ros2_client.py"
echo ""





