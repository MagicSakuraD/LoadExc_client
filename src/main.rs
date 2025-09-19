use anyhow::{bail, Context, Result};
use futures::StreamExt;
use livekit::predecessors::webrtc::video_frame::VideoRotation;
use livekit::predecessors::webrtc::video_frame_buffer::RgbaBuffer;
use livekit::predecessors::webrtc::video_source::RtcVideoSource;
use livekit::prelude::*;
use std::env;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

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
    // TODO: 在这里初始化你的 ROS2 订阅器
    // 示例：在你的 ROS 图像回调中，像这样调用推帧函数：
    //
    // let frame_data: Vec<u8> = ...; // 从 ROS msg 获取的 RGBA 数据
    // let width: u32 = ...;
    // let height: u32 = ...;
    // let timestamp_us = ...; // 获取当前时间戳
    //
    // tokio::spawn(async move {
    //     if let Err(e) = push_ros_frame_rgba(&frame_data, width, height, timestamp_us).await {
    //         warn!("Failed to push frame: {:?}", e);
    //     }
    // });
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