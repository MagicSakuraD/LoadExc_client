use anyhow::{bail, Context, Result};
use futures::StreamExt;
use livekit::prelude::*;
use std::env;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

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
    info!("connected to room: {:?}", room.name());

    // 基础事件循环（示例：打印 Track 相关事件）
    tokio::spawn(async move {
        while let Some(event) = room_events.recv().await {
            match event {
                RoomEvent::Connected => info!("room connected"),
                RoomEvent::Disconnected { reason } => warn!(?reason, "room disconnected"),
                RoomEvent::TrackPublished { publication, participant } => {
                    info!(pub_id=?publication.sid(), participant=?participant.identity(), "track published");
                }
                RoomEvent::TrackSubscribed { track, publication, participant } => {
                    info!(pub_id=?publication.sid(), participant=?participant.identity(), kind=?track.kind(), "track subscribed");
                }
                RoomEvent::TrackUnsubscribed { publication, participant } => {
                    info!(pub_id=?publication.sid(), participant=?participant.identity(), "track unsubscribed");
                }
                other => {
                    info!(?other, "room event");
                }
            }
        }
    });

    // TODO: 初始化 ROS2 订阅器（后续实现）
    // TODO: 初始化 GStreamer appsink 管线，创建并发布自定义 VideoSource（后续实现）

    // 阻塞等待 Ctrl+C 以保持进程
    tokio::signal::ctrl_c().await.ok();
    info!("shutdown");
    Ok(())
}
