#!/bin/bash
# scripts/deploy.sh - Manual deployment to cpu host
#
# Prerequisites:
#   - Docker installed locally
#   - SSH access to cpu host (159.223.224.43)
#   - Docker installed on cpu host
#
# Usage:
#   ./scripts/deploy.sh
#
# Environment variables:
#   DOMAIN - Domain name for HTTPS (optional, defaults to localhost)

set -e

HOST="cpu"
HOST_IP="159.223.224.43"
DEPLOY_DIR="/opt/vidi"

echo "=== Vidi Server Deployment ==="
echo ""

echo "Building Docker image..."
docker build -t vidi-server:latest .

echo ""
echo "Saving image..."
docker save vidi-server:latest | gzip > /tmp/vidi-server.tar.gz

echo ""
echo "Copying to $HOST..."
scp /tmp/vidi-server.tar.gz "$HOST:/tmp/"
scp docker-compose.prod.yml Caddyfile "$HOST:$DEPLOY_DIR/"

echo ""
echo "Deploying on $HOST..."
ssh "$HOST" << 'EOF'
  mkdir -p /opt/vidi
  cd /opt/vidi
  docker load < /tmp/vidi-server.tar.gz

  # Source .env if it exists (for DOMAIN variable)
  if [ -f .env ]; then
    export $(cat .env | xargs)
  fi

  DOMAIN=${DOMAIN:-localhost} docker compose -f docker-compose.prod.yml up -d
  rm /tmp/vidi-server.tar.gz

  echo ""
  echo "Container status:"
  docker ps --filter "name=vidi" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
EOF

rm /tmp/vidi-server.tar.gz

echo ""
echo "=== Deployment complete! ==="
echo ""
echo "Server is running at:"
echo "  - HTTP:  http://$HOST_IP/"
echo "  - HTTPS: https://xp.neurali.ai/ (once DNS is configured)"
echo ""
echo "DNS Configuration (GoDaddy):"
echo "  Add an A record: xp -> $HOST_IP"
echo ""
echo "To use a different domain, SSH to $HOST and set it:"
echo "  ssh $HOST 'echo DOMAIN=other.domain.com > $DEPLOY_DIR/.env'"
echo "  ssh $HOST 'cd $DEPLOY_DIR && docker compose -f docker-compose.prod.yml up -d'"
