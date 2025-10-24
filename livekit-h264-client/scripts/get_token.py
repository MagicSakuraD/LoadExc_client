#!/usr/bin/env python3
"""
自动获取LiveKit token的脚本
"""
import requests
import json
import os
import sys
from typing import Optional


def get_livekit_token(
    token_endpoint: str = "http://192.168.3.41:3000/api/token",
    room: str = "excavator-control-room",
    username: str = "python-h264-client"
) -> Optional[str]:
    """
    从token服务获取LiveKit token
    
    Args:
        token_endpoint: token服务地址
        room: 房间名
        username: 用户名
        
    Returns:
        token字符串，失败返回None
    """
    try:
        url = f"{token_endpoint}?room={room}&username={username}"
        print(f"🔑 正在获取token: {url}")
        
        response = requests.get(url, timeout=10)
        response.raise_for_status()
        
        data = response.json()
        token = data.get('token')
        
        if token:
            print(f"✅ 成功获取token: {token[:20]}...")
            return token
        else:
            print("❌ 响应中没有找到token")
            return None
            
    except requests.exceptions.RequestException as e:
        print(f"❌ 获取token失败: {e}")
        return None
    except json.JSONDecodeError as e:
        print(f"❌ 解析响应失败: {e}")
        return None
    except Exception as e:
        print(f"❌ 未知错误: {e}")
        return None


def main():
    """主函数"""
    # 从环境变量获取配置
    token_endpoint = os.getenv("LIVEKIT_TOKEN_ENDPOINT", "http://192.168.3.41:3000/api/token")
    room = os.getenv("LIVEKIT_ROOM", "excavator-control-room")
    username = os.getenv("LIVEKIT_USERNAME", "python-h264-client")
    
    # 获取token
    token = get_livekit_token(token_endpoint, room, username)
    
    if token:
        # 设置环境变量
        os.environ["LIVEKIT_TOKEN"] = token
        print(f"🔧 已设置环境变量 LIVEKIT_TOKEN")
        
        # 如果作为模块导入，返回token
        return token
    else:
        print("❌ 无法获取token，请检查token服务是否运行")
        sys.exit(1)


if __name__ == "__main__":
    main()

