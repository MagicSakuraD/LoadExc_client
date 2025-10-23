# 延迟优化指南

## 🎯 优化目标
降低从摄像头捕获到 LiveKit 推流的端到端延迟

## 📊 延迟来源分析

### 1. Python 客户端延迟源
- ❌ **时间戳绘制**: `cv2.putText()` 调用
- ❌ **控制信息叠加**: JSON 解析和文本绘制
- ❌ **延迟计算**: `time.time()` 调用和数学运算
- ❌ **队列缓冲**: 多级队列增加延迟
- ❌ **线程切换**: 捕获→处理→发布的三级流水线

### 2. Rust 客户端延迟源
- ❌ **数据复制**: `.to_vec()` 和 `copy_from_slice()`
- ❌ **JSON 处理**: 控制消息的序列化/反序列化
- ❌ **线程阻塞**: `push_i420_planes` 阻塞主循环

## ⚡ 优化措施

### Python 客户端优化

#### 标准优化版本 (`camera_publisher.py`) - 干净视频画面
```python
# 已移除的操作:
- 时间戳绘制 (datetime.now().strftime)
- 控制信息叠加 (cv2.putText for control data)
- 延迟计算 (time.time() - timestamp)
- 控制消息订阅 (已注释掉)
- 所有 cv2.putText 调用
- 所有文本叠加操作

# 默认设置:
- debug_overlay = False (默认关闭所有叠加)
- 提供纯净的摄像头画面
- 仅保留必要的颜色转换
```

#### 最小延迟版本 (`camera_publisher_minimal.py`)
```python
# 进一步优化:
- 移除所有 cv2.putText 调用
- 移除队列缓冲 (直接处理)
- 移除控制消息处理
- 移除线程切换 (捕获即发布)
- 最小等待时间 (0.001s)
```

### Rust 客户端优化

#### 数据复制优化
```rust
// 优化前 (多次复制)
let y = msg.data[0..y_size].to_vec();  // 复制1
let u = msg.data[y_size..y_size + uv_plane].to_vec();  // 复制2
let v = msg.data[y_size + uv_plane..expected].to_vec();  // 复制3

// 优化后 (零拷贝)
let y = Arc::from(&msg.data[0..y_size]);  // 零拷贝
let u = Arc::from(&msg.data[y_size..y_size + uv_plane]);  // 零拷贝
let v = Arc::from(&msg.data[y_size + uv_plane..expected]);  // 零拷贝
```

#### 异步处理优化
```rust
// 优化前 (阻塞主循环)
push_i420_planes(&y, &u, &v, width, height, ts_us).await

// 优化后 (后台处理)
tokio::task::spawn_blocking(move || {
    // 在阻塞线程中处理
    push_i420_planes(&y, &u, &v, width, height, ts_us)
});
```

## 🚀 使用方法

### 1. 标准优化模式
```bash
# 使用优化后的标准版本
./run.sh
```

### 2. 最小延迟模式
```bash
# 使用最小延迟版本
./run_minimal_latency.sh

# 或者设置环境变量
MINIMAL_MODE=1 ./run.sh
```

### 3. 仅 Rust 客户端（无 Python）
```bash
# 仅运行 Rust 客户端，假设已有视频流
./run_rust_only.sh
```

## 📈 性能对比

| 模式 | 延迟源 | 预期延迟减少 |
|------|--------|-------------|
| 原始版本 | 时间戳+控制+队列 | 基准 |
| 标准优化 | 移除时间戳+控制 | -50~100ms |
| 最小延迟 | 移除所有叠加+队列 | -100~200ms |
| 仅 Rust | 无 Python 处理 | -200~300ms |

## 🔧 进一步优化建议

### 1. 硬件优化
- 使用 USB 3.0 摄像头
- 设置合适的 FOURCC (MJPG/H264)
- 调整摄像头缓冲区大小

### 2. 系统优化
```bash
# 设置实时优先级
sudo nice -n -20 python3 camera_publisher_minimal.py

# 设置 CPU 亲和性
taskset -c 0,1 python3 camera_publisher_minimal.py
```

### 3. 网络优化
- 使用有线网络连接
- 调整 LiveKit 码率设置
- 启用 simulcast 多分辨率

### 4. 代码优化
- 使用更高效的图像格式
- 实现帧率自适应
- 添加延迟监控

## 📊 监控延迟

### Python 端监控
```python
# 在 camera_publisher_minimal.py 中添加
import time
start_time = time.time()
# ... 处理帧 ...
end_time = time.time()
latency_ms = (end_time - start_time) * 1000
print(f"处理延迟: {latency_ms:.2f}ms")
```

### Rust 端监控
```rust
// 在 main.rs 中添加
use std::time::Instant;
let start = Instant::now();
// ... 处理帧 ...
let duration = start.elapsed();
println!("处理延迟: {:?}", duration);
```

## ⚠️ 注意事项

1. **最小延迟模式**会移除所有调试信息
2. **仅 Rust 模式**需要外部视频源
3. **实时优先级**需要 root 权限
4. **网络延迟**无法通过代码优化解决

## 🎯 推荐配置

### 生产环境（最低延迟）
```bash
# 使用最小延迟模式
./run_minimal_latency.sh
```

### 开发环境（带调试）
```bash
# 使用标准优化模式
./run.sh
```

### 测试环境（仅 Rust）
```bash
# 使用仅 Rust 模式
./run_rust_only.sh
```
