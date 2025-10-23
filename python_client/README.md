# LoadExc Python 客户端

## 功能
- 订阅控制指令话题 `/controls/teleop`
- 发布摄像头视频流到 `/camera_front_wide`（I420格式）
- 在视频画面上叠加显示控制指令信息
- 支持预览窗口显示（方便对比延迟）

## 文件结构
```
python_client/
├── camera_publisher.py       # 摄像头发布器（主程序）
├── client.py                 # 旧版客户端（已废弃）
├── control_message.py        # 控制消息处理
├── video_renderer.py         # 视频渲染器
├── close_windows.py          # 关闭OpenCV窗口工具
├── install_camera.sh         # 摄像头依赖安装脚本
├── install.sh               # 通用依赖安装脚本
├── requirements.txt         # Python依赖
└── README.md                # 说明文档
```

## 使用方法

### 启动摄像头发布器
```bash
# 带预览窗口启动（默认I420格式）
python3 camera_publisher.py

# 禁用预览窗口
python3 camera_publisher.py --no-display

# 自定义摄像头参数
python3 camera_publisher.py --width 1920 --height 1080 --fps 30

# 指定摄像头设备
python3 camera_publisher.py --device /dev/video0

# 使用不同编码格式
python3 camera_publisher.py --encoding bgr8
```

### 通过 run.sh 启动
```bash
# 默认启动（带预览）
./run.sh

# 禁用预览启动
NO_PREVIEW=1 ./run.sh
```

### 清理工具
```bash
# 关闭所有OpenCV窗口和进程
python3 close_windows.py
```

## 功能说明

### 视频内容
- 实时摄像头画面
- 叠加显示控制指令信息：
  - 基础控制（档位、油门、刹车、转向）
  - 装载机控制（大臂、铲斗）
  - 延迟信息（当前时间 - 指令时间戳）
- 时间戳和帧计数显示

### 预览功能
- 实时显示摄像头画面和控制信息
- 方便对比两台LiveKit电脑之间的延迟
- 支持窗口大小调整

### 技术规格
- 分辨率：1280x720（可配置）
- 帧率：15 FPS（可配置）
- 格式：I420（默认）
- 话题：`/camera_front_wide`
- 订阅：`/controls/teleop`