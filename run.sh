#!/bin/bash

# 一键启动脚本：
# - 自动设置编译期依赖 LK_CUSTOM_WEBRTC（如未设置）
# - 程序内部使用 dotenv_override() 读取 .env（覆盖外部变量）
# - 你只需维护 .env，然后 ./run.sh 即可

set -euo pipefail

echo "🚀 启动 LoadExc_client..."

# 1) 确保 webrtc 预编译包路径（编译期环境变量）
if [ -z "${LK_CUSTOM_WEBRTC:-}" ]; then
  export LK_CUSTOM_WEBRTC="$(pwd)/.webrtc/webrtc-linux-x64-release/linux-x64-release"
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
  echo "   VIDEO_FILE=${VIDEO_FILE:-video/test.mp4}"
  echo "   LOOP_VIDEO=${LOOP_VIDEO:-true}"
  echo "   VIDEO_FPS=${VIDEO_FPS:-30}"
fi

# 2.1) 若未设置 VIDEO_FILE，则设置为你的默认视频路径
if [ -z "${VIDEO_FILE:-}" ]; then
  export VIDEO_FILE="/home/cyber-v2x/Code/RustCode/LoadExc_client/video/test.mp4"
fi

# 2.2) 若未设置 LOOP_VIDEO/VIDEO_FPS，给出默认值
export LOOP_VIDEO="${LOOP_VIDEO:-true}"
export VIDEO_FPS="${VIDEO_FPS:-30}"

# 3) 构建并运行
cargo build
cargo run "$@"


