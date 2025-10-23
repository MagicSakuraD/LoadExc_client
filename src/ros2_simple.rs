// 简化的 ROS2 客户端 - 专注于 ROS2 功能，不依赖 LiveKit
// 用于与 Python 客户端对接

use rclrs;
use rclrs::{CreateBasicExecutor, RclrsErrorFilter};
use std_msgs::msg::String as RosString;
use sensor_msgs::msg::Image as RosImage;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use anyhow::Result;

/// 统一控制消息结构（与 Python 版本保持一致）
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnifiedControlMessage {
    // 装载机专用控制
    pub rotation: f64,     // 方向盘旋转: -1 (左) to 1 (右)
    pub brake: f64,        // 刹车: 0 (松开) to 1 (踩死)
    pub throttle: f64,     // 油门: 0 (松开) to 1 (踩死)
    pub gear: String,      // 档位: 'P' | 'R' | 'N' | 'D'
    
    // 共用控制
    pub boom: f64,         // 大臂: -1 (降) to 1 (提)
    pub bucket: f64,       // 铲斗: -1 (收) to 1 (翻)
    
    // 兼容性属性
    pub left_track: f64,   // 左履带: -1 (后) to 1 (前)
    pub right_track: f64,  // 右履带: -1 (后) to 1 (前)
    pub swing: f64,        // 驾驶室旋转: -1 (左) to 1 (右)
    pub stick: f64,        // 小臂: -1 (收) to 1 (伸)
    
    // 设备类型标识
    pub device_type: String,
    pub timestamp: i64,
}

impl Default for UnifiedControlMessage {
    fn default() -> Self {
        Self {
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
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
        }
    }
}

/// 简化的 ROS2 客户端
pub struct SimpleROS2Client {
    executor: rclrs::BasicExecutor,
    node: rclrs::Node,
    control_state: Arc<Mutex<UnifiedControlMessage>>,
}

impl SimpleROS2Client {
    pub fn new() -> Result<Self> {
        // 初始化 ROS2 上下文
        let context = rclrs::Context::default_from_env()?;
        let executor = context.create_basic_executor();
        
        // 创建节点
        let node = executor.create_node("load_exc_rust_client")?;
        
        Ok(Self {
            executor,
            node,
            control_state: Arc::new(Mutex::new(UnifiedControlMessage::default())),
        })
    }
    
    /// 启动控制消息订阅者
    pub fn start_control_subscriber(&self, topic: &str) -> Result<()> {
        let control_state = self.control_state.clone();
        
        let _subscription = self.node.create_subscription::<RosString, _>(topic, move |msg: RosString| {
            if let Ok(mut state) = control_state.lock() {
                if let Ok(updated_msg) = parse_control_message(&msg.data) {
                    *state = updated_msg;
                    println!("📥 收到控制消息: {}", msg.data);
                } else {
                    println!("⚠️ 控制消息解析失败: {}", msg.data);
                }
            }
        })?;
        
        println!("✅ 控制订阅者已启动: {}", topic);
        Ok(())
    }
    
    /// 启动视频发布者
    pub fn start_video_publisher(&self, topic: &str) -> Result<()> {
        let _publisher = self.node.create_publisher::<RosImage>(topic)?;
        println!("✅ 视频发布者已启动: {}", topic);
        Ok(())
    }
    
    /// 运行 ROS2 节点
    pub fn run(&self) -> Result<()> {
        println!("🔄 启动 ROS2 节点...");
        let errors = self.executor.spin(rclrs::SpinOptions::default());
        
        if let Some(error) = errors.first_error() {
            eprintln!("❌ ROS2 运行错误: {:?}", error);
            return Err(anyhow::anyhow!("ROS2 运行失败"));
        }
        
        Ok(())
    }
    
    /// 获取当前控制状态
    pub fn get_control_state(&self) -> UnifiedControlMessage {
        self.control_state.lock().unwrap().clone()
    }
}

/// 解析控制消息
fn parse_control_message(json_data: &str) -> Result<UnifiedControlMessage> {
    let json: serde_json::Value = serde_json::from_str(json_data)?;
    
    let mut control = UnifiedControlMessage::default();
    
    // 更新时间戳
    if let Some(t) = json.get("t").and_then(|v| v.as_i64()) {
        control.timestamp = t;
    }
    
    // 根据消息类型更新相应字段
    if let Some(msg_type) = json.get("type").and_then(|v| v.as_str()) {
        match msg_type {
            "gear" => {
                if let Some(gear) = json.get("gear").and_then(|v| v.as_str()) {
                    control.gear = gear.to_string();
                }
            }
            "analog" => {
                if let Some(v_obj) = json.get("v") {
                    if let Some(rotation) = v_obj.get("rotation").and_then(|v| v.as_f64()) {
                        control.rotation = rotation;
                    }
                    if let Some(brake) = v_obj.get("brake").and_then(|v| v.as_f64()) {
                        control.brake = brake;
                    }
                    if let Some(throttle) = v_obj.get("throttle").and_then(|v| v.as_f64()) {
                        control.throttle = throttle;
                    }
                    if let Some(boom) = v_obj.get("boom").and_then(|v| v.as_f64()) {
                        control.boom = boom;
                    }
                    if let Some(bucket) = v_obj.get("bucket").and_then(|v| v.as_f64()) {
                        control.bucket = bucket;
                    }
                    if let Some(left_track) = v_obj.get("leftTrack").and_then(|v| v.as_f64()) {
                        control.left_track = left_track;
                    }
                    if let Some(right_track) = v_obj.get("rightTrack").and_then(|v| v.as_f64()) {
                        control.right_track = right_track;
                    }
                    if let Some(swing) = v_obj.get("swing").and_then(|v| v.as_f64()) {
                        control.swing = swing;
                    }
                    if let Some(stick) = v_obj.get("stick").and_then(|v| v.as_f64()) {
                        control.stick = stick;
                    }
                }
            }
            _ => {}
        }
    }
    
    Ok(control)
}

/// 主函数
pub fn main() -> Result<()> {
    println!("🚀 启动简化的 ROS2 客户端...");
    
    // 创建客户端
    let client = SimpleROS2Client::new()?;
    
    // 启动控制订阅者
    let control_topic = std::env::var("ROS_CONTROL_TOPIC").unwrap_or_else(|_| "/controls/teleop".to_string());
    client.start_control_subscriber(&control_topic)?;
    
    // 启动视频发布者
    let video_topic = std::env::var("ROS_IMAGE_TOPIC").unwrap_or_else(|_| "/camera_front_wide".to_string());
    client.start_video_publisher(&video_topic)?;
    
    println!("✅ 客户端启动成功!");
    println!("📡 订阅控制话题: {}", control_topic);
    println!("📷 发布视频话题: {}", video_topic);
    println!("🔄 开始运行...");
    
    // 运行节点
    client.run()?;
    
    println!("✅ 程序正常退出");
    Ok(())
}



