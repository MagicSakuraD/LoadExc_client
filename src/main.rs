use anyhow::{bail, Context, Result};
use futures::StreamExt;
use livekit::predecessors::webrtc::video_frame::VideoRotation;
use livekit::predecessors::webrtc::video_frame_buffer::RgbaBuffer;
use livekit::predecessors::webrtc::video_source::RtcVideoSource;
use livekit::prelude::*;
use std::env;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;
use dotenvy::dotenv;
#[cfg(feature = "ros2")]
use {
    sensor_msgs::msg::Image as RosImage,
    std::time::{SystemTime, UNIX_EPOCH},
};

// 当启用 feature "video" 时，提供一个全局存放 RtcVideoSource 的只写一次容器
#[cfg(feature = "video")]
use std::sync::OnceLock;
#[cfg(feature = "video")]
static GLOBAL_VIDEO_SOURCE: OnceLock<RtcVideoSource> = OnceLock::new();

#[tokio::main]
async fn main() -> Result<()> {
    // 日志初始化
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    info!("starting LoadExc_client");

    // 支持 .env 文件（可在项目根目录放置 .env 保存 LIVEKIT_URL/LIVEKIT_TOKEN 等）
    let _ = dotenv();

    // 从环境变量读取 LiveKit 连接参数
    let lk_url = env::var("LIVEKIT_URL").context("LIVEKIT_URL 未设置")?;
    let lk_token = env::var("LIVEKIT_TOKEN").context("LIVEKIT_TOKEN 未设置")?;

    // 连接房间
    let (room, mut room_events) = Room::connect(&lk_url, &lk_token)
        .await
        .context("连接 LiveKit 失败")?;
    info!("connected to room: '{}'", room.name());

    // 基础事件循环
    tokio::spawn(async move {
        while let Some(event) = room_events.recv().await {
            info!(?event, "room event");
        }
    });

    // 当启用 feature "video" 时，创建并发布自定义视频轨
    #[cfg(feature = "video")]
    {
        if let Err(err) = setup_and_publish_video_track(&room).await {
            warn!(error=?err, "setup video track failed");
        }
    }

    // ------------------------------------------------------------------
    // 当启用 feature "ros2" 时，启动 ROS2 订阅器，订阅 /front_camera，收到 RGBA8 图像后推送到 LiveKit
    #[cfg(feature = "ros2")]
    {
        start_ros2_camera_subscription()?;
    }
    // ------------------------------------------------------------------


    info!("Client running. Press Ctrl+C to stop.");
    // 阻塞等待 Ctrl+C 以保持进程
    tokio::signal::ctrl_c().await.ok();
    info!("shutdown");
    room.close().await?;
    Ok(())
}

// 当启用 feature "video" 时，提供视频轨道的创建与发布
#[cfg(feature = "video")]
async fn setup_and_publish_video_track(room: &Room) -> Result<()> {
    // 轨道名称可通过环境变量覆盖
    let track_name = std::env::var("VIDEO_TRACK_NAME").unwrap_or_else(|_| "ros_cam".to_string());

    // 创建一个 RtcVideoSource，这是用来手动推送视频帧的正确类型
    let source = RtcVideoSource::new();

    // 由视频源创建本地视频轨
    let local_track = LocalVideoTrack::create_video_track(&track_name, source.clone());

    // 发布轨道
    room.local_participant()
        .publish_track(TrackPublication::Local(local_track.into()))
        .await
        .context("publish video track failed")?;

    info!(track=%track_name, "published local video track");

    // 将源保存到全局以供 ROS 回调使用
    let _ = GLOBAL_VIDEO_SOURCE.set(source);

    Ok(())
}

/// 供 ROS 回调调用的推帧函数（RGBA 示例）。
/// 注意：这是一个 async 函数，因为 capture_frame 是异步操作。
#[cfg(feature = "video")]
pub async fn push_ros_frame_rgba(
    rgba_data: &[u8],
    width: u32,
    height: u32,
    timestamp_us: i64,
) -> Result<()> {
    // 从全局变量获取视频源
    let Some(source) = GLOBAL_VIDEO_SOURCE.get() else {
        warn!("no VideoSource available yet; drop frame");
        return Ok(()); // 提早返回，而不是报错
    };

    // 1. 将 RGBA 数据切片包装成 RgbaBuffer
    // 注意：这里的 stride (步长) 就是一行的字节数
    let buffer = RgbaBuffer::from_slice(
        rgba_data,
        width,
        height,
        width * 4, // Stride for RGBA is width * 4 bytes
    );

    // 2. 创建 VideoFrame
    let frame = VideoFrame {
        rotation: VideoRotation::VideoRotation0,
        timestamp_us,
        buffer: Box::new(buffer),
    };

    // 3. 异步捕获（推送）这一帧。SDK 会在后台处理编码和发送。
    source
        .capture_frame(&frame)
        .await
        .context("failed to capture frame")?;

    Ok(())
}

// ---------------- ROS2 集成（可选 feature: ros2） ----------------
#[cfg(feature = "ros2")]
fn start_ros2_camera_subscription() -> Result<()> {
    use rclrs::{Context, Node, QOS_PROFILE_DEFAULT};

    info!("starting ROS2 subscriber for /front_camera");

    // rclrs 目前采用回调 + spin 的模型，这里在新线程中 spin，不阻塞 tokio 运行时
    std::thread::spawn(|| {
        if let Err(e) = (|| -> Result<()> {
            let context = Context::new(std::env::args()).context("create ROS2 context failed")?;
            let node = Node::new(&context, "excavator_camera_subscriber").context("create ROS2 node failed")?;

            // 订阅 /front_camera
            let _sub = node
                .create_subscription::<RosImage>(
                    "/front_camera",
                    QOS_PROFILE_DEFAULT,
                    |msg: RosImage| {
                        // 收到图像后，检查编码并尝试推送
                        if msg.encoding != "rgba8" {
                            warn!(enc = %msg.encoding, "expect rgba8 encoding; drop frame");
                            return;
                        }

                        let timestamp_us = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_micros() as i64)
                            .unwrap_or(0);

                        let width = msg.width;
                        let height = msg.height;
                        let data = msg.data.clone();

                        // 交给 tokio 异步去推送，避免阻塞 rclrs 回调
                        tokio::spawn(async move {
                            #[cfg(feature = "video")]
                            {
                                if let Err(e) = push_ros_frame_rgba(&data, width, height, timestamp_us).await {
                                    warn!(?e, "push frame failed");
                                }
                            }
                        });
                    },
                )
                .context("create ROS2 subscription failed")?;

            // 在这个线程里阻塞 spin
            rclrs::spin(&node).context("spin ROS2 node failed")?;
            Ok(())
        })() {
            error!(?e, "ROS2 subscriber thread exited with error");
        }
    });

    Ok(())
}