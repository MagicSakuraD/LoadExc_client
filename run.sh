#!/bin/bash

# 一键启动脚本：
# - 自动设置编译期依赖 LK_CUSTOM_WEBRTC（如未设置）
# - 程序内部使用 dotenv_override() 读取 .env（覆盖外部变量）
# - 你只需维护 .env，然后 ./run.sh 即可

set -euo pipefail

echo "🚀 启动 LoadExc_client..."

# 1) 确保 webrtc 预编译包路径（编译期环境变量）
if [ -z "${LK_CUSTOM_WEBRTC:-}" ]; then
  export LK_CUSTOM_WEBRTC="$(pwd)/.webrtc/linux-arm64-release"
fi

# 2) 仅用于提示：读取 .env 展示关键配置（实际加载在程序内完成）
if [ -f ./.env ]; then
  # shellcheck disable=SC2046
  set -a; . ./.env; set +a
  echo "📋 .env 预览 (仅供参考，最终以程序内加载为准):"
  echo "   LIVEKIT_URL=${LIVEKIT_URL:-<unset>}"
  if [ -n "${LIVEKIT_TOKEN:-}" ]; then
    echo "   LIVEKIT_TOKEN_LEN=${#LIVEKIT_TOKEN}"
    echo "   LIVEKIT_TOKEN_HEAD=${LIVEKIT_TOKEN:0:8}..."
  else
    echo "   LIVEKIT_TOKEN_LEN=0"
  fi
  echo "   ROS_IMAGE_TOPIC=${ROS_IMAGE_TOPIC:-/camera_front_wide}"
  echo "   VIDEO_TRACK_NAME=${VIDEO_TRACK_NAME:-ros_camera_feed}"
fi

# 2.1) source ROS 与工作区环境（若存在）
if [ -f /opt/ros/humble/setup.sh ]; then
  # shellcheck disable=SC1091
  set +u
  . /opt/ros/humble/setup.sh
  set -u
fi
if [ -f "$HOME/rust_ws/install/setup.sh" ]; then
  # shellcheck disable=SC1091
  set +u
  . "$HOME/rust_ws/install/setup.sh"
  set -u
fi

# 3) 构建并运行（ROS2-only）
export ROS_IMAGE_TOPIC="${ROS_IMAGE_TOPIC:-/camera_front_wide}"

echo "🛠️  构建 (ROS2-only)"
cargo build
cargo run "$@"


