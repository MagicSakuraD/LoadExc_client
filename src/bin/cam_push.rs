use anyhow::{Context, Result};
use dotenvy::dotenv_override;
use livekit::options::{TrackPublishOptions, VideoEncoding};
use livekit::prelude::*;
use livekit::webrtc::video_frame::{I420Buffer, VideoFrame, VideoRotation};
use livekit::webrtc::video_source::native::NativeVideoSource;
use livekit::webrtc::video_source::RtcVideoSource;
use std::sync::Arc;
use tokio::time::{self, Duration, Instant};
use v4l::buffer::Type;
use v4l::io::mmap::Stream as MmapStream;
use v4l::io::traits::CaptureStream;
use v4l::prelude::*;
use v4l::video::Capture as _;

fn yuyv_to_i420_planes(src: &[u8], width: usize, height: usize) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    // YUYV 4:2:2 每 2 像素 4 字节: Y0 U Y1 V
    if src.len() < width * height * 2 {
        anyhow::bail!("YUYV 帧大小不足");
    }
    let mut y = vec![0u8; width * height];
    let mut u = vec![0u8; (width / 2) * (height / 2)];
    let mut v = vec![0u8; (width / 2) * (height / 2)];

    // 对 2x2 block 采样：
    // 行 j, j+1；列 i(偶), i+1(奇)
    for j in (0..height).step_by(2) {
        let row0_base = j * width * 2;
        let row1_base = (j + 1) * width * 2;
        for i in (0..width).step_by(2) {
            let idx0 = row0_base + i * 2; // 行 j，列 i 的起始（偶列）
            let idx1 = row1_base + i * 2; // 行 j+1，列 i 的起始（偶列）

            // 行 j 的两个像素 Y0,Y1 和共享的 U,V
            let y00 = src[idx0];
            let u0 = src[idx0 + 1];
            let y01 = src[idx0 + 2];
            let v0 = src[idx0 + 3];
            // 行 j+1 的两个像素 Y0,Y1 和共享的 U,V（同列）
            let y10 = src[idx1];
            let u1 = src[idx1 + 1];
            let y11 = src[idx1 + 2];
            let v1 = src[idx1 + 3];

            // 写入 Y 平面
            y[j * width + i] = y00;
            y[j * width + i + 1] = y01;
            y[(j + 1) * width + i] = y10;
            y[(j + 1) * width + i + 1] = y11;

            // 下采样平均得到 U/V（2 行同列平均）
            let u_avg = ((u0 as u16 + u1 as u16) / 2) as u8;
            let v_avg = ((v0 as u16 + v1 as u16) / 2) as u8;
            let uvi = (j / 2) * (width / 2) + (i / 2);
            u[uvi] = u_avg;
            v[uvi] = v_avg;
        }
    }
    Ok((y, u, v))
}

#[tokio::main]
async fn main() -> Result<()> {
    // 环境变量
    let _ = dotenv_override();
    let lk_url = std::env::var("LIVEKIT_URL").unwrap_or_else(|_| "ws://127.0.0.1:7880".to_string());
    // 若提供固定 token 则直接使用，否则尝试从 LIVEKIT_TOKEN 环境变量取值
    let lk_token = std::env::var("LIVEKIT_TOKEN")
        .or_else(|_| std::env::var("LIVEKIT_STATIC_TOKEN"))
        .context("请设置 LIVEKIT_TOKEN 或 LIVEKIT_STATIC_TOKEN 为可用的访问令牌")?;

    // 连接房间
    let (room, mut _events) = Room::connect(&lk_url, &lk_token, RoomOptions::default())
        .await
        .context("连接 LiveKit 失败")?;

    // 创建视频源与本地视频轨道
    let native_source = NativeVideoSource::default();
    let source = Arc::new(RtcVideoSource::Native(native_source));
    let track_name = std::env::var("VIDEO_TRACK_NAME").unwrap_or_else(|_| "camera0".to_string());
    let local_track = LocalVideoTrack::create_video_track(&track_name, (*source).clone());
    room
        .local_participant()
        .publish_track(
            LocalTrack::Video(local_track.clone()),
            TrackPublishOptions {
                source: TrackSource::Camera,
                simulcast: true,
                video_encoding: Some(VideoEncoding {
                    max_bitrate: 1_500_000,
                    max_framerate: 20.0,
                }),
                ..Default::default()
            },
        )
        .await
        .context("发布视频轨道失败")?;

    // 打开摄像头（/dev/video0）
    let cam_index: usize = std::env::var("CAM_INDEX").ok().and_then(|s| s.parse().ok()).unwrap_or(0);
    let mut dev = Device::new(cam_index).context("打开摄像头失败")?;

    // 配置分辨率/帧率与像素格式 YUYV
    let width: u32 = std::env::var("CAM_WIDTH").ok().and_then(|s| s.parse().ok()).unwrap_or(1280);
    let height: u32 = std::env::var("CAM_HEIGHT").ok().and_then(|s| s.parse().ok()).unwrap_or(720);
    let fps: u32 = std::env::var("CAM_FPS").ok().and_then(|s| s.parse().ok()).unwrap_or(20);

    let mut fmt = dev.format()?;
    fmt.width = width;
    fmt.height = height;
    fmt.fourcc = v4l::FourCC::new(b"YUYV");
    let fmt = dev.set_format(&fmt).context("设置摄像头格式失败")?;

    let _params = dev.params()?; // 某些平台无法程序化设置 fps，这里沿用当前设置

    // 内存映射采集流
    let mut stream = MmapStream::with_buffers(&dev, Type::VideoCapture, 4).context("创建采集流失败")?;

    // 推流循环
    let frame_interval = Duration::from_secs_f64(1.0 / (fps as f64).max(1.0));
    let mut ticker = time::interval(frame_interval);
    ticker.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

    loop {
        ticker.tick().await;
        let (buf, _meta) = match stream.next() {
            Ok(f) => f,
            Err(_) => continue,
        };

        let (y, u, v) = yuyv_to_i420_planes(buf, fmt.width as usize, fmt.height as usize)?;
        let w = fmt.width;
        let h = fmt.height;

        // 将 I420 平面数据复制到 LiveKit 的 I420Buffer 后提交
        let mut buffer = I420Buffer::new(w, h);
        let (y_dst, u_dst, v_dst) = buffer.data_mut();
        if y_dst.len() == y.len() && u_dst.len() == u.len() && v_dst.len() == v.len() {
            y_dst.copy_from_slice(&y);
            u_dst.copy_from_slice(&u);
            v_dst.copy_from_slice(&v);
        } else {
            continue;
        }

        let ts_us = Instant::now();
        let frame = VideoFrame { rotation: VideoRotation::VideoRotation0, timestamp_us: ts_us.elapsed().as_micros() as i64, buffer };
        if let RtcVideoSource::Native(native) = &*source {
            native.capture_frame(&frame);
        }
    }
}


