#!/bin/bash

# 仅运行 Rust 客户端（不启动 Python）
# 适用于上实车环境

set -euo pipefail

echo "🚀 启动 LoadExc Rust 客户端 (仅Rust) ..."

# -----------------------------
# 配置与环境
# -----------------------------

# ROS2 环境（可按需修改发行版）
if [ -f /opt/ros/humble/setup.sh ]; then
  echo "🔧 设置 ROS2 环境..."
  set +u
  . /opt/ros/humble/setup.sh
  set -u
  echo "✅ ROS2: $ROS_DISTRO"
else
  echo "⚠️ 未检测到 /opt/ros/humble/setup.sh，请确认ROS2环境已安装并手动source"
fi

# 若用户有自建工作区（按需加载）
if [ -f "$HOME/rust_ws/install/setup.sh" ]; then
  echo "🔧 加载 ROS2 工作区..."
  set +u
  . "$HOME/rust_ws/install/setup.sh"
  set -u
fi

# WebRTC 依赖（如使用自定义本地库）
if [ -z "${LK_CUSTOM_WEBRTC:-}" ]; then
  export LK_CUSTOM_WEBRTC="$(pwd)/.webrtc/linux-arm64-release"
fi

# 运行参数（环境变量可覆盖）
export ROS_IMAGE_TOPIC="${ROS_IMAGE_TOPIC:-/camera_front_wide}"
export ROS_CONTROL_TOPIC="${ROS_CONTROL_TOPIC:-/controls/teleop}"
export LIVEKIT_URL="${LIVEKIT_URL:-ws://111.186.56.118:7880}"  # 云服务器地址
export LIVEKIT_API_KEY="${LIVEKIT_API_KEY:-APIz5uJKWH46EJh}"
export LIVEKIT_API_SECRET="${LIVEKIT_API_SECRET:-YbHzvbfCmoSsYIWdX40z5wOPpIVevwmFffNvbndL60cC}"
export LIVEKIT_ROOM="${LIVEKIT_ROOM:-excavator-control-room}"
export LIVEKIT_USERNAME="${LIVEKIT_USERNAME:-heavyMachRemoteTerm}"

echo "📡 配置:"
echo "   ROS_IMAGE_TOPIC = $ROS_IMAGE_TOPIC"
echo "   ROS_CONTROL_TOPIC = $ROS_CONTROL_TOPIC"
echo "   LIVEKIT_URL = $LIVEKIT_URL"
echo "   LIVEKIT_ROOM = $LIVEKIT_ROOM"
echo "   LIVEKIT_USERNAME = $LIVEKIT_USERNAME"
echo "   LIVEKIT_API_KEY = ${LIVEKIT_API_KEY:-<not set>}"

# -----------------------------
# 构建并运行
# -----------------------------

echo "🛠️  构建 Rust 客户端..."
cargo build --release

echo "🦀 启动 Rust 客户端..."
echo "   提示: 可通过环境变量覆盖上面的配置，例如："
echo "         LIVEKIT_URL=wss://x.x.x.x:7880 LIVEKIT_API_KEY=xxx LIVEKIT_API_SECRET=yyy ./run_rust_only.sh"

# 前台运行，便于观察日志
cargo run --release

#!/bin/bash

# =============================================================================
# 单独运行 Rust 客户端脚本
# 用于排查 Rust 程序问题
# =============================================================================

set -e  # 遇到错误立即退出

echo "🚀 启动 Rust 客户端（单独运行模式）"
echo "=" * 60

# 检查必要文件
if [ ! -f "src/main.rs" ]; then
    echo "❌ 错误: 找不到 src/main.rs 文件"
    exit 1
fi

# 设置 WebRTC 环境变量
echo "🔧 设置 WebRTC 环境变量..."
export CC=clang
export CXX=clang++
export LK_CUSTOM_WEBRTC="/home/orin64/MyCode/LoadExc_client/.webrtc/linux-arm64-release"

# 设置 LiveKit 环境变量
echo "🌐 设置 LiveKit 环境变量..."
export LIVEKIT_URL="ws://192.168.3.41:7880"
export LIVEKIT_TOKEN_ENDPOINT="http://192.168.3.41:3000/api/token"
export LIVEKIT_ROOM="excavator-control-room"
export LIVEKIT_USERNAME="EQ-LDR-102"

# 设置 ROS2 环境变量
echo "🤖 设置 ROS2 环境变量..."
export ROS_DOMAIN_ID=0
export RMW_IMPLEMENTATION=rmw_cyclonedx_cpp

# 检查 WebRTC 路径
if [ ! -d "$LK_CUSTOM_WEBRTC" ]; then
    echo "⚠️  警告: WebRTC 路径不存在: $LK_CUSTOM_WEBRTC"
    echo "   请确保 WebRTC 库已正确安装"
fi

# 显示环境变量
echo ""
echo "📋 环境变量检查:"
echo "   🔧 CC=$CC"
echo "   🔧 CXX=$CXX"
echo "   🌐 LIVEKIT_URL=$LIVEKIT_URL"
echo "   🌐 LIVEKIT_TOKEN_ENDPOINT=$LIVEKIT_TOKEN_ENDPOINT"
echo "   🌐 LIVEKIT_ROOM=$LIVEKIT_ROOM"
echo "   🌐 LIVEKIT_USERNAME=$LIVEKIT_USERNAME"
echo "   📁 LK_CUSTOM_WEBRTC=$LK_CUSTOM_WEBRTC"
echo "   🤖 ROS_DOMAIN_ID=$ROS_DOMAIN_ID"
echo "   🤖 RMW_IMPLEMENTATION=$RMW_IMPLEMENTATION"
echo ""

# 检查 ROS2 环境
echo "🔍 检查 ROS2 环境..."
if ! command -v ros2 &> /dev/null; then
    echo "⚠️  警告: 未找到 ros2 命令，请确保 ROS2 已正确安装"
else
    echo "✅ ROS2 环境正常"
fi

# 检查 Rust 环境
echo "🔍 检查 Rust 环境..."
if ! command -v cargo &> /dev/null; then
    echo "❌ 错误: 未找到 cargo 命令，请确保 Rust 已正确安装"
    exit 1
else
    echo "✅ Rust 环境正常"
fi

# 编译 Rust 程序
echo ""
echo "🔨 编译 Rust 程序..."
echo "   使用编译器: CC=$CC, CXX=$CXX"
echo "   WebRTC 路径: $LK_CUSTOM_WEBRTC"
echo ""

# 清理之前的编译
echo "🧹 清理之前的编译..."
cargo clean

# 编译
echo "🔨 开始编译..."
cargo build

if [ $? -eq 0 ]; then
    echo "✅ 编译成功!"
else
    echo "❌ 编译失败!"
    exit 1
fi

echo ""
echo "🚀 启动 Rust 客户端..."
echo "   按 Ctrl+C 停止程序"
echo ""

# 运行 Rust 程序
exec cargo run
