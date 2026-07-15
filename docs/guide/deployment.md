# 🚢 生产部署

## Docker Compose 生产配置

以下是一个适合生产环境的 `docker-compose.yml` 示例：

```yaml
version: "3.8"

services:
  mcguffin:
    image: ghcr.io/deng2010/mcguffin:latest
    container_name: mcguffin
    restart: unless-stopped
    ports:
      - "127.0.0.1:3000:3000"   # 仅监听本地，用反向代理对外暴露
    environment:
      - SITE_URL=https://your-domain.com
      - ADMIN_PASSWORD=your_strong_password
    volumes:
      - mcguffin_data:/app/data
    healthcheck:
      test: ["CMD", "wget", "-qO-", "http://localhost:3000/api/health"]
      interval: 30s
      timeout: 5s
      retries: 3

volumes:
  mcguffin_data:
```

## 反向代理

### Nginx

```nginx
server {
    listen 80;
    server_name your-domain.com;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl;
    server_name your-domain.com;

    ssl_certificate /etc/letsencrypt/live/your-domain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/your-domain.com/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # WebSocket 支持（通知的实时推送）
    location /ws {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

### Caddy（自动 HTTPS）

```caddyfile
your-domain.com {
    reverse_proxy 127.0.0.1:3000
}
```

## 系统服务（非 Docker 部署）

### systemd（Linux）

创建 `/etc/systemd/system/mcguffin.service`：

```ini
[Unit]
Description=McGuffin Server
After=network.target

[Service]
Type=simple
User=mcguffin
Group=mcguffin
ExecStart=/usr/local/bin/mcguffin-server
WorkingDirectory=/usr/share/mcguffin
Environment=MCGUFFIN_DATA_DIR=/var/lib/mcguffin
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now mcguffin
sudo systemctl status mcguffin
```

### macOS launchd

创建 `~/Library/LaunchAgents/com.mcguffin.server.plist`：

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.mcguffin.server</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/mcguffin-server</string>
    </array>
    <key>WorkingDirectory</key>
    <string>/usr/share/mcguffin</string>
    <key>KeepAlive</key>
    <true/>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
```

```bash
launchctl load ~/Library/LaunchAgents/com.mcguffin.server.plist
```

## CLI 服务管理

```bash
# 启动
mcguffin start

# 停止
mcguffin stop

# 重启
mcguffin restart

# 查看状态
mcguffin status
```

## 安全建议

1. **修改默认管理员密码**：首次启动后立即更改
2. **使用 HTTPS**：生产环境必须启用 TLS
3. **绑定回环地址**：反向代理场景下，服务只监听 `127.0.0.1`
4. **定期备份**：设置合理的自动备份间隔
5. **限制文件权限**：`/usr/share/mcguffin/config.toml` 含密码明文，建议 `chmod 600`
6. **防火墙**：仅开放 80/443 端口
