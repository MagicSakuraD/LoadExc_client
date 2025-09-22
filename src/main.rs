use anyhow::{Context, Result};
use livekit::prelude::*;
use livekit::options::TrackPublishOptions;
#[cfg(feature = "ros2")]
use rclrs::{QOS_PROFILE_DEFAULT};
// Note: sensor_msgs 暂未纳入依赖；如需启用 ROS2，请在 Cargo.toml 增加消息包依赖
#[cfg(feature = "ros2")]
use sensor_msgs::msg::Image as RosImage;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;
use dotenvy::dotenv;

// No video source for now (adapting to livekit 0.7 API later)

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    info!("starting LoadExc_client");

    // Load environment variables from .env file
    let _ = dotenv();

    // Read LiveKit connection parameters from environment
    let lk_url = env::var("LIVEKIT_URL").context("LIVEKIT_URL not set")?;
    let lk_token = env::var("LIVEKIT_TOKEN").context("LIVEKIT_TOKEN not set")?;

    // Connect to room
    let (room, mut room_events) = Room::connect(&lk_url, &lk_token, RoomOptions::default())
        .await
        .context("Failed to connect to LiveKit")?;
    info!("connected to room: '{}'", room.name());

    // Skip publishing video track for now; focus on compiling with 0.7.18

    // Start ROS2 subscriber (only when feature enabled)
    #[cfg(feature = "ros2")]
    {
        info!("starting ROS2 subscriber for /front_camera (video disabled)");
        std::thread::spawn(move || {
            if let Err(e) = (|| -> Result<()> {
                let context = rclrs::Context::new(std::env::args()).context("Failed to create ROS2 context")?;
                let node = rclrs::create_node(&context, "excavator_camera_subscriber").context("Failed to create ROS2 node")?;

                let _sub = node
                    .create_subscription::<RosImage>(
                        "/front_camera",
                        QOS_PROFILE_DEFAULT,
                        move |msg: RosImage| {
                            if msg.encoding != "rgba8" {
                                warn!(enc = %msg.encoding, "expected rgba8 encoding; dropping frame");
                                return;
                            }
                            let _ = msg; // temporarily unused
                        },
                    )
                    .context("Failed to create ROS2 subscription")?;

                rclrs::spin(&node).context("Failed to spin ROS2 node")?;
                Ok(())
            })() {
                error!(?e, "ROS2 subscriber thread exited with error");
            }
        });
    }

    // Main event loop: handle room events; video disabled for now
    info!("Client running. Press Ctrl+C to stop.");
    loop {
        tokio::select! {
            Some(event) = room_events.recv() => {
                info!(?event, "room event");
                if let RoomEvent::Disconnected { .. } = event {
                    break;
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("ctrl_c received");
                break;
            }
        }
    }

    info!("shutting down");
    room.close().await?;
    Ok(())
}

// Video pipeline temporarily disabled for 0.7.18 migration