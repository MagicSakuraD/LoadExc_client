# ROS2 控制话题规格文档

## 话题信息

**话题名称**: `/controls/teleop`  
**消息类型**: `std_msgs/msg/String`  
**发布者**: LoadExc_client (Rust程序)  
**发布频率**: 约30Hz（仅在控制值变化时发布）

## 消息格式

所有消息都是JSON字符串，包含完整的装载机控制状态。消息格式统一，不再区分 `gear` 和 `analog` 类型。

### JSON消息结构

```json
{
  "rotation": 0.0,        // 方向盘旋转
  "brake": 0.5,           // 刹车
  "throttle": 0.5,        // 油门
  "gear": "D",            // 档位
  "boom": 0.0,             // 大臂
  "bucket": 0.0,          // 铲斗
  "left_track": 0.0,      // 左履带
  "right_track": 0.0,     // 右履带
  "swing": 0.0,           // 驾驶室旋转
  "stick": 0.0,           // 小臂
  "device_type": "wheel_loader",  // 设备类型
  "timestamp": 1758770906076       // 时间戳
}
```

## 控制参数详细说明

### 装载机专用控制

| 参数名 | 类型 | 范围 | 默认值 | 说明 |
|--------|------|------|--------|------|
| `rotation` | float64 | -1.0 到 1.0 | 0.0 | 方向盘旋转<br/>-1.0: 左转到底<br/>0.0: 居中<br/>1.0: 右转到底 |
| `brake` | float64 | 0.0 到 1.0 | 0.5 | 刹车踏板<br/>0.0: 完全松开<br/>1.0: 踩到底 |
| `throttle` | float64 | 0.0 到 1.0 | 0.5 | 油门踏板<br/>0.0: 完全松开<br/>1.0: 踩到底 |
| `gear` | string | "P", "R", "N", "D" | "N" | 档位选择<br/>P: 停车档<br/>R: 倒档<br/>N: 空档<br/>D: 前进档 |

### 工作装置控制

| 参数名 | 类型 | 范围 | 默认值 | 说明 |
|--------|------|------|--------|------|
| `boom` | float64 | -1.0 到 1.0 | 0.0 | 大臂控制<br/>-1.0: 大臂下降<br/>0.0: 停止<br/>1.0: 大臂上升 |
| `bucket` | float64 | -1.0 到 1.0 | 0.0 | 铲斗控制<br/>-1.0: 铲斗收起<br/>0.0: 停止<br/>1.0: 铲斗翻出 |

### 兼容性控制（履带式装载机）

| 参数名 | 类型 | 范围 | 默认值 | 说明 |
|--------|------|------|--------|------|
| `left_track` | float64 | -1.0 到 1.0 | 0.0 | 左履带<br/>-1.0: 后退<br/>0.0: 停止<br/>1.0: 前进 |
| `right_track` | float64 | -1.0 到 1.0 | 0.0 | 右履带<br/>-1.0: 后退<br/>0.0: 停止<br/>1.0: 前进 |
| `swing` | float64 | -1.0 到 1.0 | 0.0 | 驾驶室旋转<br/>-1.0: 左转<br/>0.0: 居中<br/>1.0: 右转 |
| `stick` | float64 | -1.0 到 1.0 | 0.0 | 小臂控制<br/>-1.0: 小臂收回<br/>0.0: 停止<br/>1.0: 小臂伸出 |

### 系统信息

| 参数名 | 类型 | 值 | 说明 |
|--------|------|-----|------|
| `device_type` | string | "wheel_loader" | 设备类型标识 |
| `timestamp` | int64 | 毫秒时间戳 | 消息生成时间 |

## 使用示例

### 订阅话题

```bash
# 实时查看控制消息
ros2 topic echo /controls/teleop

# 查看话题信息
ros2 topic info /controls/teleop

# 查看发布频率
ros2 topic hz /controls/teleop
```

### Python订阅示例

```python
import rclpy
from rclpy.node import Node
from std_msgs.msg import String
import json

class ControlSubscriber(Node):
    def __init__(self):
        super().__init__('control_subscriber')
        self.subscription = self.create_subscription(
            String,
            '/controls/teleop',
            self.control_callback,
            10
        )
    
    def control_callback(self, msg):
        try:
            control_data = json.loads(msg.data)
            
            # 提取控制参数
            rotation = control_data['rotation']
            brake = control_data['brake']
            throttle = control_data['throttle']
            gear = control_data['gear']
            boom = control_data['boom']
            bucket = control_data['bucket']
            
            # 处理控制逻辑
            self.process_controls(rotation, brake, throttle, gear, boom, bucket)
            
        except json.JSONDecodeError:
            self.get_logger().error('Invalid JSON in control message')
    
    def process_controls(self, rotation, brake, throttle, gear, boom, bucket):
        # 实现具体的控制逻辑
        pass

def main():
    rclpy.init()
    node = ControlSubscriber()
    rclpy.spin(node)
    rclpy.shutdown()

if __name__ == '__main__':
    main()
```

### C++订阅示例

```cpp
#include <rclcpp/rclcpp.hpp>
#include <std_msgs/msg/string.hpp>
#include <nlohmann/json.hpp>

class ControlSubscriber : public rclcpp::Node
{
public:
    ControlSubscriber() : Node("control_subscriber")
    {
        subscription_ = this->create_subscription<std_msgs::msg::String>(
            "/controls/teleop", 10,
            std::bind(&ControlSubscriber::control_callback, this, std::placeholders::_1));
    }

private:
    void control_callback(const std_msgs::msg::String::SharedPtr msg)
    {
        try {
            auto control_data = nlohmann::json::parse(msg->data);
            
            // 提取控制参数
            double rotation = control_data["rotation"];
            double brake = control_data["brake"];
            double throttle = control_data["throttle"];
            std::string gear = control_data["gear"];
            double boom = control_data["boom"];
            double bucket = control_data["bucket"];
            
            // 处理控制逻辑
            process_controls(rotation, brake, throttle, gear, boom, bucket);
            
        } catch (const std::exception& e) {
            RCLCPP_ERROR(this->get_logger(), "Invalid JSON in control message: %s", e.what());
        }
    }
    
    void process_controls(double rotation, double brake, double throttle, 
                         const std::string& gear, double boom, double bucket)
    {
        // 实现具体的控制逻辑
    }
    
    rclcpp::Subscription<std_msgs::msg::String>::SharedPtr subscription_;
};

int main(int argc, char * argv[])
{
    rclcpp::init(argc, argv);
    rclcpp::spin(std::make_shared<ControlSubscriber>());
    rclcpp::shutdown();
    return 0;
}
```

## 消息示例

### 正常行驶状态
```json
{
  "rotation": 0.0,
  "brake": 0.5,
  "throttle": 0.5,
  "gear": "D",
  "boom": 0.0,
  "bucket": 0.0,
  "left_track": 0.0,
  "right_track": 0.0,
  "swing": 0.0,
  "stick": 0.0,
  "device_type": "wheel_loader",
  "timestamp": 1758770906076
}
```

### 转向状态
```json
{
  "rotation": -0.5,
  "brake": 0.2,
  "throttle": 0.8,
  "gear": "D",
  "boom": 0.0,
  "bucket": 0.0,
  "left_track": 0.0,
  "right_track": 0.0,
  "swing": 0.0,
  "stick": 0.0,
  "device_type": "wheel_loader",
  "timestamp": 1758770906076
}
```

### 工作装置操作状态
```json
{
  "rotation": 0.0,
  "brake": 0.5,
  "throttle": 0.5,
  "gear": "D",
  "boom": 0.8,
  "bucket": -0.6,
  "left_track": 0.0,
  "right_track": 0.0,
  "swing": 0.0,
  "stick": 0.0,
  "device_type": "wheel_loader",
  "timestamp": 1758770906076
}
```

## 注意事项

1. **消息频率**: 控制消息仅在值发生变化时发布，避免不必要的网络负载
2. **数据范围**: 所有浮点数值都在 -1.0 到 1.0 范围内，超出范围的值会被截断
3. **档位安全**: 档位切换时建议先减速到安全速度
4. **工作装置**: 大臂和铲斗操作时注意安全距离，避免碰撞
5. **时间戳**: 时间戳为毫秒级Unix时间戳，可用于计算控制延迟

## 故障排除

### 常见问题

1. **话题无数据**: 检查LoadExc_client程序是否正常运行
2. **JSON解析错误**: 确认消息格式正确，检查字符编码
3. **控制无响应**: 检查控制值范围是否在有效区间内
4. **延迟过高**: 检查网络连接和系统负载

### 调试命令

```bash
# 检查话题状态
ros2 topic list | grep controls

# 查看话题详细信息
ros2 topic info /controls/teleop

# 监控消息频率
ros2 topic hz /controls/teleop

# 查看最近的消息
ros2 topic echo /controls/teleop --once
```

---

**文档版本**: 1.0  
**最后更新**: 2025-01-25  
**维护者**: LoadExc_client 开发团队
