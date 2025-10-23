#!/bin/bash

# =============================================================================
# Rust 客户端调试脚本
# 用于排查 Rust 程序问题
# =============================================================================

echo "🐛 Rust 客户端调试模式"
echo "========================"

# 设置环境变量
export CC=clang
export CXX=clang++
export LK_CUSTOM_WEBRTC="/home/orin64/MyCode/LoadExc_client/.webrtc/linux-arm64-release"
export LIVEKIT_URL="ws://192.168.3.41:7880"
export LIVEKIT_TOKEN_ENDPOINT="http://192.168.3.41:3000/api/token"
export LIVEKIT_ROOM="excavator-control-room"
export LIVEKIT_USERNAME="EQ-LDR-102"
export ROS_DOMAIN_ID=0
export RMW_IMPLEMENTATION=rmw_cyclonedx_cpp

echo "🔧 环境变量:"
echo "   CC=$CC"
echo "   CXX=$CXX"
echo "   LK_CUSTOM_WEBRTC=$LK_CUSTOM_WEBRTC"
echo "   LIVEKIT_URL=$LIVEKIT_URL"
echo ""

# 编译并运行
echo "🔨 编译并运行 Rust 客户端..."
cargo run






