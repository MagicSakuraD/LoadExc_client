#!/bin/bash

# 测试调试输出的脚本
echo "🧪 LoadExc_client 调试测试脚本"
echo "================================"

# 设置环境变量
export LIVEKIT_URL="wss://your-livekit-server.com"
export LIVEKIT_TOKEN="your-token-here"
export VIDEO_TRACK_NAME="ros_cam"
export LK_CUSTOM_WEBRTC="/home/cyber-v2x/Code/RustCode/LoadExc_client/.webrtc/webrtc-linux-x64-release/linux-x64-release"

# 视频文件路径和循环设置
export VIDEO_FILE="/home/cyber-v2x/Code/RustCode/LoadExc_client/video/test.mp4"
export LOOP_VIDEO="true"

echo "📋 环境变量设置:"
echo "   LIVEKIT_URL: $LIVEKIT_URL"
echo "   LIVEKIT_TOKEN: ${LIVEKIT_TOKEN:0:20}..."
echo "   VIDEO_TRACK_NAME: $VIDEO_TRACK_NAME"
echo "   LK_CUSTOM_WEBRTC: $LK_CUSTOM_WEBRTC"
echo "   VIDEO_FILE: $VIDEO_FILE"
echo "   LOOP_VIDEO: $LOOP_VIDEO"
echo ""

echo "🚀 启动 LoadExc_client (带调试输出)..."
echo "   按 Ctrl+C 停止程序"
echo ""

# 运行程序
cargo run
