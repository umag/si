#!/usr/bin/env bash
# Quick start script for SI self-hosting
# This script guides you through the initial setup

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}"
cat << "EOF"
   _____ _____    _____      _  __     _    _           _   _
  / ____|_   _|  / ____|    | |/ _|   | |  | |         | | (_)
 | (___   | |   | (___   ___| | |_ ___| |__| | ___  ___| |_ _ _ __   __ _
  \___ \  | |    \___ \ / _ \ |  _|_  |  __  |/ _ \/ __| __| | '_ \ / _` |
  ____) |_| |_   ____) |  __/ | |  / /| |  | | (_) \__ \ |_| | | | | (_| |
 |_____/|_____|  |_____/ \___|_|_| /___|_|  |_|\___/|___/\__|_|_| |_|\__, |
                                                                       __/ |
                                                                      |___/
EOF
echo -e "${NC}"

# Check prerequisites
echo -e "${BLUE}Checking prerequisites...${NC}"

# Check Docker
if ! command -v docker &> /dev/null; then
    echo -e "${RED}✗ Docker is not installed${NC}"
    echo "Please install Docker: https://docs.docker.com/get-docker/"
    exit 1
else
    echo -e "${GREEN}✓ Docker is installed${NC}"
fi

# Check Docker Compose
if ! docker compose version &> /dev/null; then
    echo -e "${RED}✗ Docker Compose is not installed${NC}"
    echo "Please install Docker Compose v2: https://docs.docker.com/compose/install/"
    exit 1
else
    echo -e "${GREEN}✓ Docker Compose is installed${NC}"
fi

# Check if Docker daemon is running
if ! docker info &> /dev/null; then
    echo -e "${RED}✗ Docker daemon is not running${NC}"
    echo "Please start Docker and try again"
    exit 1
else
    echo -e "${GREEN}✓ Docker daemon is running${NC}"
fi

# Check available disk space
AVAILABLE_SPACE=$(df -BG . | tail -1 | awk '{print $4}' | sed 's/G//')
if [ "$AVAILABLE_SPACE" -lt 50 ]; then
    echo -e "${YELLOW}⚠ Warning: Less than 50GB available disk space${NC}"
    echo -e "${YELLOW}  You have ${AVAILABLE_SPACE}GB available. Build may fail.${NC}"
fi

echo ""

# Step 1: Environment configuration
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Step 1: Environment Configuration${NC}"
echo -e "${BLUE}========================================${NC}"

if [ ! -f .env ]; then
    echo -e "${YELLOW}Creating .env file from template...${NC}"
    cp .env.selfhost.example .env

    # Generate secure passwords
    POSTGRES_PASS=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-32)
    SPICEDB_KEY=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-32)
    ZED_PASS=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-32)
    JWT_SECRET=$(openssl rand -base64 64 | tr -d "=+/" | cut -c1-64)

    # Update .env with generated passwords
    sed -i.bak "s|POSTGRES_PASSWORD=.*|POSTGRES_PASSWORD=${POSTGRES_PASS}|" .env
    sed -i.bak "s|SPICEDB_PRESHARED_KEY=.*|SPICEDB_PRESHARED_KEY=${SPICEDB_KEY}|" .env
    sed -i.bak "s|ZED_KEYRING_PASSWORD=.*|ZED_KEYRING_PASSWORD=${ZED_PASS}|" .env
    sed -i.bak "s|JWT_SECRET=.*|JWT_SECRET=${JWT_SECRET}|" .env
    rm -f .env.bak

    echo -e "${GREEN}✓ Generated secure passwords in .env${NC}"
else
    echo -e "${GREEN}✓ .env file already exists${NC}"
fi

echo -e "\n${GREEN}Authentication: LOCAL_AUTH_MODE is enabled by default (no Auth0 required)${NC}"
echo -e "${YELLOW}Note: For production with multiple users, set LOCAL_AUTH_MODE=false and configure Auth0${NC}"
read -p "Press Enter to continue..."

# Step 2: Start local registry
echo -e "\n${BLUE}========================================${NC}"
echo -e "${BLUE}Step 2: Local Docker Registry${NC}"
echo -e "${BLUE}========================================${NC}"

if docker ps --filter "name=registry" --format "{{.Names}}" | grep -q "^registry$"; then
    echo -e "${GREEN}✓ Local Docker registry is already running${NC}"
else
    echo -e "${YELLOW}Starting local Docker registry...${NC}"
    docker run -d -p 5000:5000 --name registry --restart=always registry:2
    sleep 2
    echo -e "${GREEN}✓ Local registry started${NC}"
fi

# Step 3: Build images
echo -e "\n${BLUE}========================================${NC}"
echo -e "${BLUE}Step 3: Build Docker Images${NC}"
echo -e "${BLUE}========================================${NC}"

echo -e "${YELLOW}This will take 20-60 minutes depending on your system...${NC}"
echo -e ""
read -p "Start building? (y/N): " -n 1 -r
echo

if [[ $REPLY =~ ^[Yy]$ ]]; then
    ./scripts/build-and-push-local.sh
else
    echo -e "${YELLOW}Skipping build. You can build later with:${NC}"
    echo -e "  ${YELLOW}./scripts/build-and-push-local.sh${NC}"
    exit 0
fi

# Step 4: Start services
echo -e "\n${BLUE}========================================${NC}"
echo -e "${BLUE}Step 4: Start Services${NC}"
echo -e "${BLUE}========================================${NC}"

read -p "Start services now? (y/N): " -n 1 -r
echo

if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}Starting services...${NC}"
    docker compose -f docker-compose.selfhost.yml up -d

    echo -e "\n${YELLOW}Waiting for services to be healthy...${NC}"
    sleep 10

    # Show status
    docker compose -f docker-compose.selfhost.yml ps
else
    echo -e "${YELLOW}Skipping service start. You can start later with:${NC}"
    echo -e "  ${YELLOW}docker compose -f docker-compose.selfhost.yml up -d${NC}"
    exit 0
fi

# Summary
echo -e "\n${GREEN}========================================${NC}"
echo -e "${GREEN}✓ Setup Complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo -e "Services are starting up. Access them at:"
echo -e "  ${BLUE}Web UI:        ${GREEN}http://localhost:8080${NC}"
echo -e "  ${BLUE}Auth Portal:   ${GREEN}http://localhost:9000${NC}"
echo -e "  ${BLUE}API:           ${GREEN}http://localhost:5156/api/${NC}"
echo -e "  ${BLUE}Documentation: ${GREEN}http://localhost:5173${NC}"
echo ""
echo -e "Useful commands:"
echo -e "  View logs:     ${YELLOW}docker compose -f docker-compose.selfhost.yml logs -f${NC}"
echo -e "  Stop services: ${YELLOW}docker compose -f docker-compose.selfhost.yml down${NC}"
echo -e "  Restart:       ${YELLOW}docker compose -f docker-compose.selfhost.yml restart${NC}"
echo ""
echo -e "Or use the Makefile:"
echo -e "  ${YELLOW}make -f Makefile.selfhost help${NC}"
echo ""
echo -e "${BLUE}Happy building! 🚀${NC}"
