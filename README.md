## LoadExc_client 使用说明

- 在项目根目录创建 `.env` 文件，内容示例：
```
LIVEKIT_URL=wss://your-livekit-host
LIVEKIT_TOKEN=your_jwt_token
VIDEO_TRACK_NAME=ros_cam
RUST_LOG=info
```

- 运行（仅 LiveKit 视频轨）：
```
cargo run --features video
```

- 运行（LiveKit + ROS2 订阅 `/front_camera`）：
```
cargo run --features "video ros2"
```

- ROS2 图像要求：`sensor_msgs/msg/Image`，编码 `rgba8`，建议分辨率 1920x1080。

### 最小摄像头推流（cam_push）

新增二进制 `cam_push`：直接从本机摄像头（/dev/videoX）采集 YUYV，转换为 I420，并推送到 LiveKit 房间。

依赖：
- 需要可访问的 `/dev/videoX`（默认 index 0）
- 环境变量提供 LiveKit 连接参数

环境变量（建议使用 .env 文件）：

```bash
# .env 示例（也可参考仓库中的 .env.example）
LIVEKIT_URL=ws://<host>:7880
LIVEKIT_TOKEN=<your_access_token>
VIDEO_TRACK_NAME=camera0
CAM_INDEX=0
CAM_WIDTH=1280
CAM_HEIGHT=720
CAM_FPS=20
RUST_LOG=info
```

构建：

```bash
cargo build --bin cam_push --release
```

运行（自动从 .env 读取环境变量）：

```bash
cargo run --bin cam_push --release
```

说明：
- 若无静态 token，请在服务端签发或参考 LiveKit 文档生成访问令牌。
- 摄像头像素格式使用 YUYV（4:2:2），程序内转换到 I420（4:2:0）后推入 LiveKit。
- 默认开启 simulcast，最大码率与帧率可在 `src/bin/cam_push.rs` 中的 `TrackPublishOptions` 调整。
