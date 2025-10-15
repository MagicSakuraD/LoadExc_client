use anyhow::{Context, Result};
use dotenvy::dotenv_override;
use livekit::options::{TrackPublishOptions, VideoEncoding};
use livekit::prelude::*;
use livekit::webrtc::video_frame::{I420Buffer, VideoFrame, VideoRotation};
use livekit::webrtc::video_source::native::NativeVideoSource;
use livekit::webrtc::video_source::RtcVideoSource;
use std::sync::Arc;
use tokio::time::{self, Duration, Instant};
use chrono::Local;
use v4l::buffer::Type;
use v4l::io::mmap::Stream as MmapStream;
use v4l::io::traits::CaptureStream;
use v4l::prelude::*;
use v4l::video::Capture as _;
use sdl2::{pixels::PixelFormatEnum, rect::Rect};

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

// 5x7 简易字体（每个字符 5 列、7 行，bit1 表示填充）
const FONT_5X7: [[u8; 7]; 12] = [
    // '0'..'9', ':', '.'
    [0b01110,0b10001,0b10011,0b10101,0b11001,0b10001,0b01110], // 0
    [0b00100,0b01100,0b00100,0b00100,0b00100,0b00100,0b01110], // 1
    [0b01110,0b10001,0b00001,0b00010,0b00100,0b01000,0b11111], // 2
    [0b11110,0b00001,0b00001,0b00110,0b00001,0b00001,0b11110], // 3
    [0b00010,0b00110,0b01010,0b10010,0b11111,0b00010,0b00010], // 4
    [0b11111,0b10000,0b11110,0b00001,0b00001,0b10001,0b01110], // 5
    [0b00110,0b01000,0b10000,0b11110,0b10001,0b10001,0b01110], // 6
    [0b11111,0b00001,0b00010,0b00100,0b01000,0b01000,0b01000], // 7
    [0b01110,0b10001,0b10001,0b01110,0b10001,0b10001,0b01110], // 8
    [0b01110,0b10001,0b10001,0b01111,0b00001,0b00010,0b01100], // 9
    [0b00000,0b00100,0b00100,0b00000,0b00100,0b00100,0b00000], // ':'
    [0b00000,0b00000,0b00000,0b00000,0b00000,0b00100,0b00000], // '.'
];

fn glyph_index(ch: char) -> Option<usize> {
    match ch {
        '0'..='9' => Some((ch as u8 - b'0') as usize),
        ':' => Some(10),
        '.' => Some(11),
        _ => None,
    }
}

fn draw_text_i420(y: &mut [u8], u: &mut [u8], v: &mut [u8], width: usize, height: usize, x0: usize, y0: usize, scale: usize, text: &str) {
    let y_white: u8 = 235; // 白色亮度
    let u_neutral: u8 = 128; // 中性色度
    let v_neutral: u8 = 128;
    let glyph_w = 5usize;
    let glyph_h = 7usize;
    let gap = 1usize; // 字符间距

    let mut cx = x0;
    for ch in text.chars() {
        if let Some(idx) = glyph_index(ch) {
            for gy in 0..glyph_h {
                if y0 + gy >= height { continue; }
                let row = FONT_5X7[idx][gy];
                for gx in 0..glyph_w {
                    if cx + gx >= width { continue; }
                    let bit = (row >> (glyph_w - 1 - gx)) & 1;
                    if bit == 1 {
                        // 放大绘制 scale x scale
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let py = y0 + gy * scale + sy;
                                let px = cx + gx * scale + sx;
                                if py >= height || px >= width { continue; }
                                y[py * width + px] = y_white;
                                let uvi = (py / 2) * (width / 2) + (px / 2);
                                if uvi < u.len() { u[uvi] = u_neutral; }
                                if uvi < v.len() { v[uvi] = v_neutral; }
                            }
                        }
                    }
                }
            }
            cx += (glyph_w + gap) * scale;
        } else {
            cx += (glyph_w + gap) * scale; // 未知字符占位
        }
        if cx >= width { break; }
    }
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

    // 配置分辨率/帧率与像素格式（默认 YUYV，可通过 CAM_FOURCC 覆盖）
    let width: u32 = std::env::var("CAM_WIDTH").ok().and_then(|s| s.parse().ok()).unwrap_or(1280);
    let height: u32 = std::env::var("CAM_HEIGHT").ok().and_then(|s| s.parse().ok()).unwrap_or(720);
    let fps: u32 = std::env::var("CAM_FPS").ok().and_then(|s| s.parse().ok()).unwrap_or(20);
    let fourcc_str = std::env::var("CAM_FOURCC").unwrap_or_else(|_| "YUYV".to_string());
    let mut fourcc_bytes = [0u8; 4];
    for (i, b) in fourcc_str.bytes().take(4).enumerate() { fourcc_bytes[i] = b; }

    let mut fmt = dev.format()?;
    fmt.width = width;
    fmt.height = height;
    fmt.fourcc = v4l::FourCC::new(&fourcc_bytes);
    let fmt = dev.set_format(&fmt).context("设置摄像头格式失败")?;

    let _params = dev.params()?; // 某些平台无法程序化设置 fps，这里沿用当前设置

    // 内存映射采集流
    let mut stream = MmapStream::with_buffers(&dev, Type::VideoCapture, 4).context("创建采集流失败")?;

    // 推流循环
    let frame_interval = Duration::from_secs_f64(1.0 / (fps as f64).max(1.0));
    let mut ticker = time::interval(frame_interval);
    ticker.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

    // 可选本地预览（SDL2），设置 PREVIEW=1 启用
    let enable_preview = std::env::var("PREVIEW").ok().map(|v| v == "1" || v.to_lowercase() == "true").unwrap_or(false);
    let mut _sdl_ctx_opt: Option<sdl2::Sdl> = None;
    let mut canvas_opt: Option<sdl2::render::Canvas<sdl2::video::Window>> = None;
    // 我们仅持有 canvas；纹理在每帧内创建并释放，避免生命周期问题
    if enable_preview {
        if let Ok(sdl) = sdl2::init() {
            if let Ok(video) = sdl.video() {
                if let Ok(window) = video
                    .window("cam_push preview", width, height)
                    .position_centered()
                    .opengl()
                    .build()
                {
                    if let Ok(canvas) = window.into_canvas().accelerated().present_vsync().build() {
                        // 重要：保持创建顺序与持有顺序：canvas -> texture_creator -> texture
                        canvas_opt = Some(canvas);
                        _sdl_ctx_opt = Some(sdl);
                    } else {
                        _sdl_ctx_opt = Some(sdl);
                    }
                } else {
                    _sdl_ctx_opt = Some(sdl);
                }
            } else {
                _sdl_ctx_opt = Some(sdl);
            }
        }
    }

    loop {
        ticker.tick().await;
        let (buf, _meta) = match stream.next() {
            Ok(f) => f,
            Err(_) => continue,
        };

        // 根据实际 fourcc 处理: 支持 YUYV 和 MJPG
        let (mut y, mut u, mut v) = if fmt.fourcc == v4l::FourCC::new(b"YUYV") {
            yuyv_to_i420_planes(buf, fmt.width as usize, fmt.height as usize)?
        } else if fmt.fourcc == v4l::FourCC::new(b"MJPG") {
            // MJPG: 解码 JPEG 为 RGB，再转换到 I420
            match jpeg_decoder::Decoder::new(buf).decode() {
                Ok(rgb) => {
                    let w_us = fmt.width as usize;
                    let h_us = fmt.height as usize;
                    if rgb.len() != w_us * h_us * 3 { continue; }
                    // RGB -> I420
                    let mut y = vec![0u8; w_us * h_us];
                    let mut u = vec![0u8; (w_us/2) * (h_us/2)];
                    let mut v = vec![0u8; (w_us/2) * (h_us/2)];
                    for j in (0..h_us).step_by(2) {
                        for i in (0..w_us).step_by(2) {
                            let mut u_acc: i32 = 0;
                            let mut v_acc: i32 = 0;
                            for dy in 0..2 { for dx in 0..2 {
                                let x = i + dx; let yj = j + dy;
                                let idx = (yj*w_us + x) * 3;
                                let r = rgb[idx] as f32;
                                let g = rgb[idx+1] as f32;
                                let b = rgb[idx+2] as f32;
                                let y_val = (0.257*r + 0.504*g + 0.098*b + 16.0).round() as i32;
                                y[yj*w_us + x] = y_val.clamp(0,255) as u8;
                                let u_val = (-0.148*r - 0.291*g + 0.439*b + 128.0).round() as i32;
                                let v_val = (0.439*r - 0.368*g - 0.071*b + 128.0).round() as i32;
                                u_acc += u_val; v_acc += v_val;
                            }}
                            let uvi = (j/2)*(w_us/2) + (i/2);
                            u[uvi] = (u_acc/4).clamp(0,255) as u8;
                            v[uvi] = (v_acc/4).clamp(0,255) as u8;
                        }
                    }
                    (y,u,v)
                }
                Err(_) => { continue }
            }
        } else {
            // 其它格式暂不支持
            continue;
        };
        let w = fmt.width;
        let h = fmt.height;

        // 叠加时间戳文本（本地时间）
        let now = Local::now();
        let ts_text = now.format("%H:%M:%S.%3f").to_string();
        // 位置与缩放可配置
        let pos = std::env::var("TIMESTAMP_POS").unwrap_or_else(|_| "tl".to_string()); // tl|tr|bl|br
        let scale: usize = std::env::var("TIMESTAMP_SCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(3);
        let margin = 16usize;
        let text_px_w = (5 + 1) * scale * ts_text.len();
        let text_px_h = 7 * scale;
        let (mut x, mut ytop) = (margin, margin);
        let w_us = w as usize;
        let h_us = h as usize;
        match pos.as_str() {
            "tr" => { x = w_us.saturating_sub(text_px_w + margin); ytop = margin; }
            "bl" => { x = margin; ytop = h_us.saturating_sub(text_px_h + margin); }
            "br" => { x = w_us.saturating_sub(text_px_w + margin); ytop = h_us.saturating_sub(text_px_h + margin); }
            _ => {}
        }
        draw_text_i420(&mut y, &mut u, &mut v, w as usize, h as usize, x, ytop, scale, &ts_text);

        // 本地预览：使用 SDL2 IYUV 纹理（与 I420 顺序一致）
        if let Some(canvas) = &mut canvas_opt {
            let y_pitch = w as usize;
            let uv_pitch = (w / 2) as usize;
            // 每帧创建纹理，作用域内使用完即丢弃，避免生命周期问题
            if let Ok(mut tex) = canvas.texture_creator().create_texture_streaming(PixelFormatEnum::IYUV, w, h) {
                let _ = tex.update_yuv(
                None,
                &y, y_pitch,
                &u, uv_pitch,
                &v, uv_pitch,
                );
                canvas.clear();
                let _ = canvas.copy(&tex, None, Some(Rect::new(0, 0, w, h)));
                canvas.present();
            }
        }

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


