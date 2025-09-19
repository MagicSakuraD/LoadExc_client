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
