// main.rs

use anyhow::{Context, Result};
use dotenvy::dotenv_override;
use livekit::prelude::*;
use livekit::options::{TrackPublishOptions, VideoEncoding};
use livekit::webrtc::video_frame::{VideoFrame, VideoRotation, I420Buffer};
use livekit::webrtc::video_source::{RtcVideoSource, native::NativeVideoSource};
use std::env;
use std::sync::Arc;
use std::sync::mpsc as std_mpsc;
use tokio::sync::mpsc;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use serde_json::Value;
use reqwest;

// ROS2 crates
use rclrs;
use rclrs::{CreateBasicExecutor, RclrsErrorFilter};
use sensor_msgs::msg::Image as RosImage;
use std_msgs::msg::String as RosString;

// 全局视频源，用于从 GStreamer 线程安全地推送视频帧
static GLOBAL_VIDEO_SOURCE: std::sync::OnceLock<Arc<RtcVideoSource>> = std::sync::OnceLock::new();

// 全局控制状态，用于合并 gear 和 analog 消息
static GLOBAL_CONTROL_STATE: std::sync::OnceLock<std::sync::Mutex<UnifiedControlMessage>> = std::sync::OnceLock::new();

// 定义一个统一的帧消息，以便未来扩展（例如，如果也需要处理 RGBA）
enum FrameMsg {
    I420 {
        y: Arc<[u8]>,  // 使用Arc<[u8]>避免Vec分配和拷贝
        u: Arc<[u8]>,  // 使用Arc<[u8]>避免Vec分配和拷贝
        v: Arc<[u8]>,  // 使用Arc<[u8]>避免Vec分配和拷贝
        width: u32,
        height: u32,
        ts_us: i64,
    },
}

/// 控制消息（从 LiveKit DataChannel 转 ROS2，统一处理所有类型）
enum ControlMsg {
    Data {
        data: Arc<Vec<u8>>,
        reliable: bool,
    },
}

/// 统一控制消息结构（合并 gear 和 analog）
#[derive(serde::Serialize, serde::Deserialize)]
struct UnifiedControlMessage {
    // 装载机专用控制
    rotation: f64,     // 方向盘旋转: -1 (左) to 1 (右)
    brake: f64,        // 刹车: 0 (松开) to 1 (踩死)
    throttle: f64,     // 油门: 0 (松开) to 1 (踩死)
    gear: String,      // 档位: 'P' | 'R' | 'N' | 'D'
    
    // 共用控制
    boom: f64,         // 大臂: -1 (降) to 1 (提)
    bucket: f64,       // 铲斗: -1 (收) to 1 (翻)
    
    // 兼容性属性（设为默认值）
    left_track: f64,   // 左履带: -1 (后) to 1 (前)
    right_track: f64,  // 右履带: -1 (后) to 1 (前)
    swing: f64,        // 驾驶室旋转: -1 (左) to 1 (右)
    stick: f64,        // 小臂: -1 (收) to 1 (伸)
    
    // 设备类型标识
    device_type: String, // 设备类型
    timestamp: i64,    // 时间戳
}

// 仅保留 ROS2 订阅路径（无 GStreamer 路径）

fn start_ros2_image_subscriber(tx: mpsc::Sender<FrameMsg>, topic: String) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        // 基于当前 rclrs 版本的推荐写法：Context -> Executor -> Node -> Subscription -> spin
        let mut executor = match rclrs::Context::default_from_env() {
            Ok(ctx) => ctx.create_basic_executor(),
            Err(e) => {
                eprintln!("ROS2 Context init failed: {:?}", e);
                return;
            }
        };
        let node = match executor.create_node("lk_ros_client") {
            Ok(n) => n,
            Err(e) => {
                eprintln!("ROS2 Node create failed: {:?}", e);
                return;
            }
        };
        let tx_sub = tx.clone();
        let _subscription = match node.create_subscription::<RosImage, _>(&topic, move |msg: RosImage| {
            let width = msg.width;
            let height = msg.height;
            let step = msg.step;
            let enc = msg.encoding.to_lowercase();
            let data_len = msg.data.len();

            if enc != "i420" {
                println!(
                    "⚠️  收到非 I420 编码: enc='{}' (len={}), w={}, h={}, step={}",
                    msg.encoding, data_len, width, height, step
                );
                return;
            }

            let ts_us = (msg.header.stamp.sec as i64) * 1_000_000 + (msg.header.stamp.nanosec as i64) / 1_000;


            // 处理 I420 格式（原有逻辑）
            let y_size = (width as usize) * (height as usize);
            let uv_plane = (width as usize * height as usize) / 4;
            let expected = y_size + 2 * uv_plane;

            if data_len < expected {
                println!(
                    "⚠️  I420 数据长度不足: got={}, expected={} (w={}, h={}, step={})",
                    data_len, expected, width, height, step
                );
                return;
            }

            if step != width {
                println!(
                    "⚠️  发现 stride(每行步长) 与 width 不一致: step={} != width={}，需按行拷贝平面。",
                    step, width
                );
            }

            // 零拷贝优化：使用Arc::from避免to_vec()复制
            let y = Arc::from(&msg.data[0..y_size]);
            let u = Arc::from(&msg.data[y_size..y_size + uv_plane]);
            let v = Arc::from(&msg.data[y_size + uv_plane..expected]);

            // 视频帧日志过多，开发阶段关闭此高频打印，如需调试可启用

            if let Err(e) = tx_sub.try_send(FrameMsg::I420 { 
                y, 
                u, 
                v, 
                width, 
                height, 
                ts_us 
            }) {
                println!("⚠️  发送到通道失败(满?): {:?}", e);
            }
        }) {
            Ok(s) => {
                println!("✅ ROS2 订阅已创建: topic='{}'", topic);
                s
            },
            Err(e) => {
                eprintln!("ROS2 Subscription create failed: {:?}", e);
                return;
            }
        };

        println!("🔄 即将进入 ROS2 spin()");
        let errs = executor.spin(rclrs::SpinOptions::default());
        if let Err(e) = errs.first_error() {
            eprintln!("ROS2 spin failed: {:?}", e);
        }
    })
}

/// 启动 ROS2 控制话题发布线程（统一处理所有控制消息）
fn start_ros2_controls_publisher(
    rx: std_mpsc::Receiver<ControlMsg>,
    control_topic: String,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        // 初始化 ROS2 上下文与节点
        let executor = match rclrs::Context::default_from_env() {
            Ok(ctx) => ctx.create_basic_executor(),
            Err(e) => {
                eprintln!("ROS2 Context init failed (controls): {:?}", e);
                return;
            }
        };
        let node = match executor.create_node("lk_ros_controls_bridge") {
            Ok(n) => n,
            Err(e) => {
                eprintln!("ROS2 Node create failed (controls): {:?}", e);
                return;
            }
        };

        // 创建统一控制发布者
        let pub_control = match node.create_publisher::<RosString>(&control_topic) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Create publisher '{}' failed: {:?}", control_topic, e);
                return;
            }
        };

        println!("✅ ROS2 控制发布器已创建: '{}'", control_topic);

        // 不需要持续 spin 发布，也可偶尔 spin 一下处理内部事件
        loop {
            match rx.recv() {
                Ok(ControlMsg::Data { data, reliable }) => {
                    // 直接从字节切片获得UTF-8视图，避免分配与复制
                    let payload = match std::str::from_utf8(data.as_ref()) {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("⚠️  控制消息非 UTF-8，丢弃: {:?}", e);
                            continue;
                        }
                    };

                    // 解析并合并控制消息
                    if let Ok(unified_msg) = parse_and_merge_control_message(&payload) {
                        // 统一处理所有控制消息，不区分类型
                        if payload.len() <= 512 {
                            println!("🎮 统一控制消息: reliable={} data={}", reliable, payload);
                        } else {
                            println!("🎮 统一控制消息: reliable={} data_len={}", reliable, payload.len());
                        }

                        let mut msg = RosString::default();
                        msg.data = serde_json::to_string(&unified_msg).unwrap_or_else(|_| payload.to_string());
                        if let Err(e) = pub_control.publish(msg) {
                            eprintln!("⚠️  发布 '{}' 失败: {:?}", control_topic, e);
                        }
                    } else {
                        eprintln!("⚠️  解析控制消息失败，丢弃: {}", payload);
                    }
                }
                Err(_) => {
                    println!("🛑 控制通道已关闭，结束 ROS2 控制发布线程");
                    break;
                }
            }
            // 让出执行权，避免忙等
            std::thread::yield_now();
        }
    })
}

/// 解析并合并控制消息（gear 和 analog）
fn parse_and_merge_control_message(payload: &str) -> Result<UnifiedControlMessage, Box<dyn std::error::Error>> {
    let json: Value = serde_json::from_str(payload)?;
    
    // 获取全局控制状态
    let state = GLOBAL_CONTROL_STATE.get_or_init(|| {
        std::sync::Mutex::new(UnifiedControlMessage {
            rotation: 0.0,
            brake: 0.0,
            throttle: 0.0,
            gear: "N".to_string(),
            boom: 0.0,
            bucket: 0.0,
            left_track: 0.0,
            right_track: 0.0,
            swing: 0.0,
            stick: 0.0,
            device_type: "wheel_loader".to_string(),
            timestamp: 0,
        })
    });
    
    let mut current_state = state.lock().unwrap();
    
    // 更新时间戳
    if let Some(t) = json.get("t").and_then(|v| v.as_i64()) {
        current_state.timestamp = t;
    }
    
    // 根据消息类型更新相应字段
    if let Some(msg_type) = json.get("type").and_then(|v| v.as_str()) {
        match msg_type {
            "gear" => {
                if let Some(gear) = json.get("gear").and_then(|v| v.as_str()) {
                    current_state.gear = gear.to_string();
                }
            }
            "analog" => {
                if let Some(v_obj) = json.get("v") {
                    if let Some(rotation) = v_obj.get("rotation").and_then(|v| v.as_f64()) {
                        current_state.rotation = rotation;
                    }
                    if let Some(brake) = v_obj.get("brake").and_then(|v| v.as_f64()) {
                        current_state.brake = brake;
                    }
                    if let Some(throttle) = v_obj.get("throttle").and_then(|v| v.as_f64()) {
                        current_state.throttle = throttle;
                    }
                    if let Some(boom) = v_obj.get("boom").and_then(|v| v.as_f64()) {
                        current_state.boom = boom;
                    }
                    if let Some(bucket) = v_obj.get("bucket").and_then(|v| v.as_f64()) {
                        current_state.bucket = bucket;
                    }
                    if let Some(left_track) = v_obj.get("leftTrack").and_then(|v| v.as_f64()) {
                        current_state.left_track = left_track;
                    }
                    if let Some(right_track) = v_obj.get("rightTrack").and_then(|v| v.as_f64()) {
                        current_state.right_track = right_track;
                    }
                    if let Some(swing) = v_obj.get("swing").and_then(|v| v.as_f64()) {
                        current_state.swing = swing;
                    }
                    if let Some(stick) = v_obj.get("stick").and_then(|v| v.as_f64()) {
                        current_state.stick = stick;
                    }
                }
            }
            _ => {}
        }
    }
    
    // 返回当前状态的副本
    Ok(UnifiedControlMessage {
        rotation: current_state.rotation,
        brake: current_state.brake,
        throttle: current_state.throttle,
        gear: current_state.gear.clone(),
        boom: current_state.boom,
        bucket: current_state.bucket,
        left_track: current_state.left_track,
        right_track: current_state.right_track,
        swing: current_state.swing,
        stick: current_state.stick,
        device_type: current_state.device_type.clone(),
        timestamp: current_state.timestamp,
    })
}

/// 将已是 I420 格式的帧平面数据推送到 LiveKit (同步版本)
fn push_i420_planes_sync(
    y_plane: &Arc<[u8]>,
    u_plane: &Arc<[u8]>,
    v_plane: &Arc<[u8]>,
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
        // 使用更高效的内存操作，减少复制开销
        unsafe {
            std::ptr::copy_nonoverlapping(
                y_plane.as_ptr(),
                y_data.as_mut_ptr(),
                y_plane.len()
            );
            std::ptr::copy_nonoverlapping(
                u_plane.as_ptr(),
                u_data.as_mut_ptr(),
                u_plane.len()
            );
            std::ptr::copy_nonoverlapping(
                v_plane.as_ptr(),
                v_data.as_mut_ptr(),
                v_plane.len()
            );
        }
    } else {
        println!(
            "⚠️  平面尺寸不匹配，丢弃帧: dst(Y,U,V)=({},{},{}), src(Y,U,V)=({},{},{}) w={}, h={}",
            y_data.len(), u_data.len(), v_data.len(), y_plane.len(), u_plane.len(), v_plane.len(), width, height
        );
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
    let _ = dotenv_override().ok();

    // 读取 LiveKit 连接参数（默认云服务器地址，可被环境变量覆盖）
    let lk_url = env::var("LIVEKIT_URL").unwrap_or_else(|_| "ws://111.186.56.118:7880".to_string());
    println!("   🔧 LIVEKIT_URL={}", lk_url);

    // 支持两种认证方式：动态Token签发 或 直接API Key/Secret
    let endpoint = env::var("LIVEKIT_TOKEN_ENDPOINT").unwrap_or_default();
    let api_key = env::var("LIVEKIT_API_KEY").unwrap_or_default();
    let api_secret = env::var("LIVEKIT_API_SECRET").unwrap_or_default();
    
    let lk_token = if !endpoint.is_empty() {
        // 方式1：动态Token签发
        let room = env::var("LIVEKIT_ROOM").unwrap_or_else(|_| "excavator-control-room".to_string());
        let username = env::var("LIVEKIT_USERNAME").unwrap_or_else(|_| "heavyMachRemoteTerm".to_string());

        println!("   🌐 正在从 LIVEKIT_TOKEN_ENDPOINT 获取动态 Token...\n       endpoint={} room={} username={}", endpoint, room, username);

        let url = format!("{}?room={}&username={}", endpoint, urlencoding::encode(&room), urlencoding::encode(&username));
        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .context("请求 LIVEKIT_TOKEN_ENDPOINT 失败")?;

        if !resp.status().is_success() {
            anyhow::bail!(format!("LIVEKIT_TOKEN_ENDPOINT 返回非 2xx: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await.context("解析 token JSON 失败")?;
        let token = json.get("token").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if token.is_empty() {
            anyhow::bail!("LIVEKIT_TOKEN_ENDPOINT 未返回 token 字段")
        }
        println!("   ✅ LIVEKIT_TOKEN: [hidden] (fetched)");
        token
    } else if !api_key.is_empty() && !api_secret.is_empty() {
        // 方式2：直接使用API Key/Secret（需要外部Token生成服务）
        let room = env::var("LIVEKIT_ROOM").unwrap_or_else(|_| "excavator-control-room".to_string());
        let username = env::var("LIVEKIT_USERNAME").unwrap_or_else(|_| "heavyMachRemoteTerm".to_string());
        
        println!("   🔑 使用 API Key/Secret 模式...\n       room={} username={}", room, username);
        println!("   ⚠️  注意：需要配置 LIVEKIT_TOKEN_ENDPOINT 来生成Token");
        anyhow::bail!("请设置 LIVEKIT_TOKEN_ENDPOINT 来生成Token，或使用动态Token签发方式")
    } else {
        anyhow::bail!("请设置 LIVEKIT_TOKEN_ENDPOINT 或 LIVEKIT_API_KEY+LIVEKIT_API_SECRET")
    };

    // --- LiveKit 连接和轨道创建 ---
    println!("🔗 正在连接到 LiveKit 房间...");
    let (room, mut room_events) = Room::connect(&lk_url, &lk_token, RoomOptions::default())
        .await
        .context("连接到 LiveKit 失败")?;
    info!("Connected to room: '{}'", room.name());
    println!("   ✅ 成功连接到房间: '{}'", room.name());

    println!("🎥 创建并发布视频轨道...");
    let track_name = env::var("VIDEO_TRACK_NAME").unwrap_or_else(|_| "ros_camera_feed".to_string());
    let native_source = NativeVideoSource::default();
    let source = RtcVideoSource::Native(native_source);
    let local_track = LocalVideoTrack::create_video_track(&track_name, source.clone());
    
    room.local_participant()
        .publish_track(
            LocalTrack::Video(local_track.clone()),
            TrackPublishOptions { 
                source: TrackSource::Camera, 
                simulcast: true,  // 启用simulcast多分辨率流
                video_encoding: Some(VideoEncoding {
                    max_bitrate: 2_000_000,  // 2Mbps最大码率（保持高清）
                    max_framerate: 20.0,     // 20fps帧率（适合远程操作）
                }),
                ..Default::default() 
            }
        )
        .await
        .context("发布视频轨道失败")?;
    
    info!(track = %track_name, "Published local video track");
    println!("   ✅ 视频轨道 '{}' 发布成功", track_name);
    let _ = GLOBAL_VIDEO_SOURCE.set(Arc::new(source));

    // --- 仅 ROS2 视频源 ---
    let (tx, mut rx) = mpsc::channel::<FrameMsg>(64);
    let topic = std::env::var("ROS_IMAGE_TOPIC").unwrap_or_else(|_| "/camera_front_wide".to_string());
    println!("🛰️  使用 ROS2 图像话题: {}", topic);
    let _handle = start_ros2_image_subscriber(tx, topic);

    // --- ROS2 控制发布器（接收 LiveKit DataChannel -> 统一发布到 ROS2 话题） ---
    let (ctl_tx, ctl_rx) = std_mpsc::channel::<ControlMsg>();
    let ros_control_topic = std::env::var("ROS_CONTROL_TOPIC").unwrap_or_else(|_| "/controls/teleop".to_string());
    let _ctl_handle = start_ros2_controls_publisher(ctl_rx, ros_control_topic.clone());


    // --- 主事件循环 ---
    println!("🔄 进入主事件循环 (按 Ctrl+C 停止)");
    loop {
        tokio::select! {
            // 监听 LiveKit 房间事件
            Some(event) = room_events.recv() => {
                info!(?event, "Received room event");
                match event {
                    RoomEvent::Disconnected { .. } => {
                        println!("   ❌ 房间连接已断开，程序即将退出。");
                        break;
                    }
                    // DataChannel 数据（统一处理所有类型）
                    RoomEvent::DataReceived { participant: _, payload, topic, kind } => {
                        let reliable = format!("{:?}", kind).to_lowercase().contains("reliable");
                        println!("📡 收到数据通道消息: topic={:?}, reliable={}, len={}", topic, reliable, payload.len());
                        // 统一透传所有控制消息到 ROS2 发布线程
                        let _ = ctl_tx.send(ControlMsg::Data { data: payload, reliable });
                    }
                    _ => {}
                }
            }
            // 监听从 ROS2 图像订阅来的新视频帧（静默处理，避免刷屏）
            Some(msg) = rx.recv() => {
                let FrameMsg::I420 { y, u, v, width, height, ts_us } = msg;
                // 在后台阻塞线程执行拷贝与提交，避免阻塞主异步循环
                tokio::task::spawn_blocking(move || {
                    // 直接调用同步函数，避免 block_on 套娃
                    if let Err(e) = push_i420_planes_sync(&y, &u, &v, width, height, ts_us) {
                        warn!("Failed to push frame to LiveKit: {:?}", e);
                    }
                });
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
    // 无 GStreamer 管道
    
    // 关闭 LiveKit 房间连接
    room.close().await?;
    
    println!("✅ 程序正常退出");
    Ok(())
}