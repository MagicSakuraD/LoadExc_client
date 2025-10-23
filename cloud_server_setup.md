# 云服务器 LiveKit 配置指南

## 已修改的配置

### 1. Rust 客户端 (`src/main.rs`)
- ✅ 默认 LiveKit URL 改为云服务器：`ws://111.186.56.118:7880`
- ✅ 支持两种认证方式：
  - 动态 Token 签发（通过 `LIVEKIT_TOKEN_ENDPOINT`）
  - API Key/Secret 模式（需要外部 Token 生成服务）

### 2. 运行脚本 (`run_rust_only.sh`)
- ✅ 默认配置云服务器参数
- ✅ 支持环境变量覆盖

## 使用方法

### 方法1：使用动态 Token 签发（推荐）
```bash
# 设置 Token 生成端点
export LIVEKIT_TOKEN_ENDPOINT="http://111.186.56.118:3000/api/token"
export LIVEKIT_URL="ws://111.186.56.118:7880"
export LIVEKIT_ROOM="excavator-control-room"
export LIVEKIT_USERNAME="heavyMachRemoteTerm"

# 运行
./run_rust_only.sh
```

### 方法2：使用 API Key/Secret（需要 Token 生成服务）
```bash
# 设置 API 凭据
export LIVEKIT_URL="ws://111.186.56.118:7880"
export LIVEKIT_API_KEY="APIz5uJKWH46EJh"
export LIVEKIT_API_SECRET="YbHzvbfCmoSsYIWdX40z5wOPpIVevwmFffNvbndL60cC"
export LIVEKIT_ROOM="excavator-control-room"
export LIVEKIT_USERNAME="heavyMachRemoteTerm"
export LIVEKIT_TOKEN_ENDPOINT="http://111.186.56.118:3000/api/token"

# 运行
./run_rust_only.sh
```

## 云服务器配置要求

### 1. 防火墙设置
```bash
# 开放 LiveKit 端口
sudo ufw allow 7880  # WebSocket
sudo ufw allow 7881  # HTTP API
sudo ufw allow 50000:60000/udp  # TURN/STUN
```

### 2. LiveKit 服务器配置
确保 `config.yaml` 包含：
```yaml
port: 7880
rtc:
  udp_port: 50000
  use_external_ip: true
  stun_servers:
    - stun:stun.l.google.com:19302
keys:
  APIz5uJKWH46EJh: YbHzvbfCmoSsYIWdX40z5wOPpIVevwmFffNvbndL60cC
```

### 3. Token 生成服务（可选）
如果需要动态 Token 签发，可以部署一个简单的 Token 生成服务：

```javascript
// server.js (Node.js)
const express = require('express');
const { AccessToken } = require('livekit-server-sdk');

const app = express();
const API_KEY = 'APIz5uJKWH46EJh';
const API_SECRET = 'YbHzvbfCmoSsYIWdX40z5wOPpIVevwmFffNvbndL60cC';

app.get('/api/token', (req, res) => {
  const { room, username } = req.query;
  
  const token = new AccessToken(API_KEY, API_SECRET, {
    identity: username,
    name: username,
  });
  
  token.addGrant({
    roomJoin: true,
    room: room,
    canPublish: true,
    canSubscribe: true,
    canPublishData: true,
  });
  
  res.json({ token: token.toJwt() });
});

app.listen(3000);
```

## 测试连接

```bash
# 检查编译
cargo check

# 运行测试
./run_rust_only.sh
```

## 故障排除

1. **连接失败**：检查防火墙和网络连通性
2. **Token 错误**：验证 API Key/Secret 配置
3. **权限问题**：确保 Token 包含正确的权限
4. **端口冲突**：确保 7880/7881 端口未被占用
