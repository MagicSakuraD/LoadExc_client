#!/bin/bash

# 只测试视频处理功能的脚本（不连接 LiveKit）
echo "🎬 视频处理测试脚本"
echo "===================="

# 设置环境变量
export VIDEO_FILE="/home/cyber-v2x/Code/RustCode/LoadExc_client/video/test.mp4"
export LOOP_VIDEO="true"
export LK_CUSTOM_WEBRTC="/home/cyber-v2x/Code/RustCode/LoadExc_client/.webrtc/webrtc-linux-x64-release/linux-x64-release"

# 设置假的 LiveKit 连接（程序会跳过连接）
export LIVEKIT_URL="wss://fake-server.com"
export LIVEKIT_TOKEN="fake-token"
export SKIP_LIVEKIT_CONNECTION="true"

echo "📋 环境变量设置:"
echo "   VIDEO_FILE: $VIDEO_FILE"
echo "   LOOP_VIDEO: $LOOP_VIDEO"
echo "   SKIP_LIVEKIT_CONNECTION: $SKIP_LIVEKIT_CONNECTION"
echo "   LK_CUSTOM_WEBRTC: $LK_CUSTOM_WEBRTC"
echo ""

# 检查视频文件是否存在
if [ ! -f "$VIDEO_FILE" ]; then
    echo "❌ 视频文件不存在: $VIDEO_FILE"
    echo "请确保视频文件存在，或修改 VIDEO_FILE 环境变量"
    exit 1
fi

echo "✅ 视频文件存在: $VIDEO_FILE"
echo ""

echo "🚀 启动视频处理测试..."
echo "   按 Ctrl+C 停止程序"
echo ""

# 运行程序
cargo run
