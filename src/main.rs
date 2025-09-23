// main.rs

use anyhow::{anyhow, Context, Result};
use dotenvy::dotenv;
use livekit::prelude::*;
use livekit::options::TrackPublishOptions;
use livekit::webrtc::video_frame::{VideoFrame, VideoRotation, I420Buffer};
use livekit::webrtc::video_source::{RtcVideoSource, native::NativeVideoSource};
use std::env;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn, error};
use tracing_subscriber::EnvFilter;

// GStreamer crates
use gstreamer as gst;
use gstreamer::prelude::*; // Trait extensions for GStreamer elements
use gstreamer_app as gst_app;
use gstreamer_app::prelude::*; // Trait extensions for AppSink
use gstreamer_video as gst_video;

// 全局视频源，用于从 GStreamer 线程安全地推送视频帧
static GLOBAL_VIDEO_SOURCE: std::sync::OnceLock<Arc<RtcVideoSource>> = std::sync::OnceLock::new();

// 定义一个统一的帧消息，以便未来扩展（例如，如果也需要处理 RGBA）
enum FrameMsg {
    I420 {
        y: Vec<u8>,
        u: Vec<u8>,
        v: Vec<u8>,
        width: u32,
        height: u32,
        ts_us: i64,
    },
}

/// 设置并启动 GStreamer 管道
/// 这个函数会处理所有 GStreamer 相关的初始化工作
fn setup_gstreamer_pipeline(tx: mpsc::Sender<FrameMsg>) -> Result<gst::Pipeline> {
    println!("🎬 启动 GStreamer 文件解码...");
    gst::init().context("Failed to initialize GStreamer")?;

    let video_path = env::var("VIDEO_FILE").unwrap_or_else(|_| "video/test.mp4".to_string());
    println!("   📄 输入文件: {}", &video_path);
    if !Path::new(&video_path).exists() {
        anyhow::bail!("❌ 输入文件不存在，请检查 VIDEO_FILE 路径: {}", video_path);
    }
    
    let loop_video = env::var("LOOP_VIDEO").unwrap_or_else(|_| "true".to_string()).parse::<bool>().unwrap_or(true);
    println!("   🔄 循环播放: {}", if loop_video { "启用" } else { "禁用" });

    // 构建 GStreamer 管道描述字符串
    // filesrc -> decodebin -> videoconvert -> video/x-raw,format=I420 -> appsink
    let fps: u32 = env::var("VIDEO_FPS").ok().and_then(|v| v.parse().ok()).unwrap_or(30);
    let pipeline_desc = format!(
        "filesrc location=\"{}\" ! decodebin ! videoconvert ! videorate ! video/x-raw,format=I420,framerate={}/1 ! appsink name=sink emit-signals=true sync=true max-buffers=2 drop=true",
        video_path, fps
    );
    println!("   ⚙️  GStreamer Pipeline: {}", pipeline_desc);

    let pipeline = gst::parse::launch(&pipeline_desc)
        .context("Failed to build GStreamer pipeline from description")?;
    
    let pipeline = pipeline
        .dynamic_cast::<gst::Pipeline>()
        .map_err(|_| anyhow!("Failed to cast GstElement to GstPipeline"))?;

    let sink = pipeline
        .by_name("sink")
        .ok_or_else(|| anyhow!("Could not find element 'sink' in the pipeline"))?
        .dynamic_cast::<gst_app::AppSink>()
        .map_err(|_| anyhow!("Sink element is not an AppSink"))?;

    // 设置 AppSink 的属性与回调函数，当有新帧可用时，GStreamer 会调用这个闭包
    sink.set_property("sync", &true);
    sink.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            .new_sample(move |appsink| {
                let sample = appsink.pull_sample().map_err(|_| {
                    warn!("Could not pull sample from appsink");
                    gst::FlowError::Eos
                })?;

                let buffer = sample.buffer().ok_or_else(|| {
                    warn!("GStreamer sample did not contain a buffer");
                    gst::FlowError::Error
                })?;
                
                let info = sample.caps()
                    .and_then(|c| gst_video::VideoInfo::from_caps(c).ok())
                    .ok_or_else(|| {
                        warn!("GStreamer sample caps did not contain video info");
                        gst::FlowError::Error
                    })?;

                // 从 buffer 中提取 I420 的 Y, U, V 三个平面
                let map = gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info)
                    .map_err(|_| {
                        warn!("Failed to map GStreamer buffer as video frame");
                        gst::FlowError::Error
                    })?;
                
                let y = map.plane_data(0).unwrap_or_default().to_vec();
                let u = map.plane_data(1).unwrap_or_default().to_vec();
                let v = map.plane_data(2).unwrap_or_default().to_vec();
                
                // 使用系统时间作为时间戳，保证按实时节奏推送
                let ts_us = {
                    use std::time::{SystemTime, UNIX_EPOCH};
                    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                    now.as_micros() as i64
                };

                // 通过通道将帧数据发送到主 Tokio 循环
                let _ = tx.try_send(FrameMsg::I420 {
                    y, u, v,
                    width: info.width(),
                    height: info.height(),
                    ts_us,
                });

                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );

    // 【重要】设置 GStreamer 消息总线监听，以实现健壮的循环播放
    if loop_video {
        let bus = pipeline.bus().context("Failed to get pipeline bus")?;
        let pipeline_weak = pipeline.downgrade(); // 使用弱引用以避免循环引用

        // 在一个单独的线程中监听总线消息，不会阻塞主循环
        std::thread::spawn(move || {
            for msg in bus.iter_timed(gst::ClockTime::NONE) {
                // 仅在 pipeline 仍然存在时处理消息
                if let Some(pipeline) = pipeline_weak.upgrade() {
                    match msg.view() {
                        // 当收到 EOS (End-of-Stream) 消息时...
                        gst::MessageView::Eos(_) => {
                            info!("GStreamer EOS received, seeking to beginning for loop.");
                            // 将播放位置重置到开头，实现无缝循环
                            if let Err(e) = pipeline.seek_simple(gst::SeekFlags::FLUSH, gst::ClockTime::ZERO) {
                                warn!("Failed to seek pipeline to the beginning: {:?}", e);
                            }
                        }
                        gst::MessageView::Error(err) => {
                            error!(
                                "Error from GStreamer pipeline: {}, debug: {}",
                                err.error(),
                                err.debug().unwrap_or_else(|| "No debug info".into())
                            );
                            break; // 出现错误时退出监听线程
                        }
                        _ => {}
                    }
                } else {
                    break; // 如果 pipeline 被销毁，则退出线程
                }
            }
        });
    }

    Ok(pipeline)
}

/// 将已是 I420 格式的帧平面数据推送到 LiveKit
async fn push_i420_planes(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: u32,
    height: u32,
    timestamp_us: i64,
) -> Result<()> {
    let Some(source) = GLOBAL_VIDEO_SOURCE.get() else {
        warn!("VideoSource not available, dropping frame");
        return Ok(());
    };

    let mut buffer = I420Buffer::new(width, height);
    let (y_data, u_data, v_data) = buffer.data_mut();
    
    // 确保我们的数据能够放入 LiveKit 的 buffer 中
    if y_data.len() == y_plane.len() && u_data.len() == u_plane.len() && v_data.len() == v_plane.len() {
        y_data.copy_from_slice(y_plane);
        u_data.copy_from_slice(u_plane);
        v_data.copy_from_slice(v_plane);
    } else {
        warn!("Plane data size mismatch, dropping frame");
        return Ok(());
    }

    let frame = VideoFrame {
        rotation: VideoRotation::VideoRotation0,
        timestamp_us,
        buffer,
    };

    if let RtcVideoSource::Native(native_source) = &**source {
        native_source.capture_frame(&frame);
    } else {
        warn!("Unsupported video source type");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    info!("Starting LoadExc_client");
    println!("🚀 LoadExc_client 启动中...");
    println!("📋 环境变量检查:");

    // 优先使用 .env 中的配置（覆盖已存在的环境变量）
    let _ = dotenvy::dotenv_override().ok();

    // 读取 LiveKit 连接参数
    let lk_url = env::var("LIVEKIT_URL").context("环境变量 LIVEKIT_URL 未设置")?;
    let lk_token = env::var("LIVEKIT_TOKEN").context("环境变量 LIVEKIT_TOKEN 未设置")?;

    println!("   ✅ LIVEKIT_URL: {}", lk_url);
    println!("   ✅ LIVEKIT_TOKEN: [hidden]");

    // --- LiveKit 连接和轨道创建 ---
    println!("🔗 正在连接到 LiveKit 房间...");
    let (room, mut room_events) = Room::connect(&lk_url, &lk_token, RoomOptions::default())
        .await
        .context("连接到 LiveKit 失败")?;
    info!("Connected to room: '{}'", room.name());
    println!("   ✅ 成功连接到房间: '{}'", room.name());

    println!("🎥 创建并发布视频轨道...");
    let track_name = env::var("VIDEO_TRACK_NAME").unwrap_or_else(|_| "gstreamer_feed".to_string());
    let native_source = NativeVideoSource::default();
    let source = RtcVideoSource::Native(native_source);
    let local_track = LocalVideoTrack::create_video_track(&track_name, source.clone());
    
    room.local_participant()
        .publish_track(
            LocalTrack::Video(local_track.clone()),
            TrackPublishOptions { source: TrackSource::Camera, ..Default::default() }
        )
        .await
        .context("发布视频轨道失败")?;
    
    info!(track = %track_name, "Published local video track");
    println!("   ✅ 视频轨道 '{}' 发布成功", track_name);
    let _ = GLOBAL_VIDEO_SOURCE.set(Arc::new(source));

    // --- GStreamer 设置 ---
    let (tx, mut rx) = mpsc::channel::<FrameMsg>(4); // 创建通道，容量为 4
    let pipeline = setup_gstreamer_pipeline(tx)?;

    // 启动 GStreamer 管道
    pipeline.set_state(gst::State::Playing)
        .context("无法将 GStreamer 管道设置为 Playing 状态")?;
    println!("   ✅ GStreamer 管道已启动");


    // --- 主事件循环 ---
    println!("🔄 进入主事件循环 (按 Ctrl+C 停止)");
    let mut frame_count = 0;
    loop {
        tokio::select! {
            // 监听 LiveKit 房间事件
            Some(event) = room_events.recv() => {
                info!(?event, "Received room event");
                if let RoomEvent::Disconnected { .. } = event {
                    println!("   ❌ 房间连接已断开，程序即将退出。");
                    break;
                }
            }
            // 监听从 GStreamer 传来的新视频帧
            Some(msg) = rx.recv() => {
                frame_count += 1;
                match msg {
                    FrameMsg::I420 { y, u, v, width, height, ts_us } => {
                        if frame_count % 100 == 0 { // 每 100 帧打印一次日志，避免刷屏
                             println!("   🎬 正在处理第 {} 帧: {}x{}", frame_count, width, height);
                        }
                       
                        if let Err(e) = push_i420_planes(&y, &u, &v, width, height, ts_us).await {
                            warn!("Failed to push frame to LiveKit: {:?}", e);
                        }
                    }
                }
            }
            // 监听 Ctrl+C 信号以优雅地关闭
            _ = tokio::signal::ctrl_c() => {
                info!("Ctrl+C received, shutting down.");
                println!("\n🛑 收到 Ctrl+C 信号，正在退出...");
                break;
            }
        }
    }

    // --- 优雅地关闭 ---
    println!("🔄 正在关闭连接和管道...");
    
    // 停止 GStreamer 管道
    if let Err(e) = pipeline.set_state(gst::State::Null) {
        warn!("Failed to set pipeline to Null state: {}", e);
    }
    
    // 关闭 LiveKit 房间连接
    room.close().await?;
    
    println!("✅ 程序正常退出");
    Ok(())
}