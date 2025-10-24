#!/bin/bash
# LiveKit I420摄像头客户端运行脚本
set -euo pipefail

echo "🚀 启动LiveKit I420摄像头客户端..."

# 设置环境变量
export LIVEKIT_URL="${LIVEKIT_URL:-ws://111.186.56.118:7880}"
export LIVEKIT_TOKEN="${LIVEKIT_TOKEN:-}"
export CAMERA_DEVICE="${CAMERA_DEVICE:-/dev/video0}"

# 检查环境变量，如果没有则尝试自动获取
if [ -z "$LIVEKIT_TOKEN" ]; then
    echo "🔑 未找到LIVEKIT_TOKEN，尝试自动获取..."
    export LIVEKIT_TOKEN=$(uv run scripts/get_token.py 2>/dev/null)
    
    if [ -z "$LIVEKIT_TOKEN" ]; then
        echo "❌ 错误: 无法获取LIVEKIT_TOKEN"
        echo "请确保token服务正在运行: http://192.168.3.41:3000/api/token"
        echo "或手动设置环境变量:"
        echo " export LIVEKIT_TOKEN='your_token_here'"
        exit 1
    fi
fi

# 检查依赖
echo "🔍 检查依赖..."

# 检查摄像头
if ! ls /dev/video* >/dev/null 2>&1; then
    echo "❌ 未找到摄像头设备"
    exit 1
fi

echo "🎥 可用设备: $(ls /dev/video* 2>/dev/null | xargs)"
echo "🎯 使用设备: ${CAMERA_DEVICE}"

# 检查摄像头是否被占用
echo "🔍 检查摄像头资源占用情况..."
if lsof "${CAMERA_DEVICE}" >/dev/null 2>&1; then
    echo "⚠️  摄像头 ${CAMERA_DEVICE} 被占用，正在释放..."
    
    # 获取占用摄像头的进程
    PIDS=$(lsof -t "${CAMERA_DEVICE}" 2>/dev/null || true)
    if [ -n "$PIDS" ]; then
        echo "🔧 释放摄像头资源: PID $PIDS"
        kill -9 $PIDS 2>/dev/null || true
        sleep 1
        echo "✅ 摄像头资源已释放"
    fi
else
    echo "✅ 摄像头资源可用"
fi

# 检查OpenCV和LiveKit依赖
if ! python3 -c "import cv2, livekit" 2>/dev/null; then
    echo "❌ 缺少必要的Python依赖"
    echo "请运行: uv sync 或 pip install opencv-python livekit"
    exit 1
fi

echo "✅ 依赖检查通过"

# 运行客户端
echo "🐍 启动Python摄像头客户端（I420格式，低延迟）..."
echo "📊 特性: YUYV原生格式 → I420推流 → 控制信息叠加"
python3 test_camera_i420.py
