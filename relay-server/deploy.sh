#!/bin/bash
# ============================================================
# MCP2P Relay Server — Deploy Script для Ubuntu 24.04
# VPS: oskarlolpo-vpn (2.26.54.53)
# ============================================================
#
# Использование:
#   1. Скопируйте этот скрипт на VPS:
#      scp deploy.sh root@2.26.54.53:/root/
#   2. Запустите:
#      ssh root@2.26.54.53 "bash /root/deploy.sh"
#
# Или просто выполните команды вручную по порядку.
# ============================================================

set -e

echo "=== MCP2P Relay: Установка зависимостей ==="
apt-get update -y
apt-get install -y curl build-essential nginx certbot python3-certbot-nginx

echo "=== MCP2P Relay: Установка Rust ==="
if ! command -v rustup &>/dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
fi

echo "=== MCP2P Relay: Клонирование и сборка ==="
cd /root
if [ ! -d "relay-server" ]; then
    mkdir -p relay-server/src
fi

# Файлы Cargo.toml и src/main.rs нужно скопировать заранее через scp
cd relay-server
cargo build --release

echo "=== MCP2P Relay: Установка бинарника ==="
cp target/release/mcp2p-relay /usr/local/bin/mcp2p-relay
chmod +x /usr/local/bin/mcp2p-relay

echo "=== MCP2P Relay: Создание systemd сервиса ==="
cat > /etc/systemd/system/mcp2p-relay.service << 'EOF'
[Unit]
Description=MCP2P WebSocket Relay Server
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/mcp2p-relay
Environment=RUST_LOG=mcp2p_relay=info
Restart=always
RestartSec=3
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable mcp2p-relay
systemctl start mcp2p-relay

echo "=== MCP2P Relay: Настройка Nginx ==="
cat > /etc/nginx/sites-available/mcp2p-relay << 'NGINX'
server {
    listen 80;
    server_name oskarlolpo-vpn.play2go.cloud;

    location /ws {
        proxy_pass http://127.0.0.1:8443;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_read_timeout 3600s;
        proxy_send_timeout 3600s;
        proxy_buffering off;
        tcp_nodelay on;
    }

    location / {
        return 200 'MCP2P Relay OK';
        add_header Content-Type text/plain;
    }
}
NGINX

ln -sf /etc/nginx/sites-available/mcp2p-relay /etc/nginx/sites-enabled/
nginx -t && systemctl reload nginx

echo "=== MCP2P Relay: Получение TLS сертификата ==="
certbot --nginx -d oskarlolpo-vpn.play2go.cloud --non-interactive --agree-tos --email artyom@example.com || {
    echo "ВНИМАНИЕ: Certbot не смог получить сертификат."
    echo "Возможно, домен не указывает на этот сервер."
    echo "Попробуйте вручную: certbot --nginx -d oskarlolpo-vpn.play2go.cloud"
}

echo ""
echo "============================================"
echo " MCP2P Relay Server установлен!"
echo " Статус: systemctl status mcp2p-relay"
echo " Логи:   journalctl -u mcp2p-relay -f"
echo " URL:    wss://oskarlolpo-vpn.play2go.cloud/ws"
echo "============================================"
