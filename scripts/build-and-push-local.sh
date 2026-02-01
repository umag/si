#!/usr/bin/env bash
# Build and push SI services to local Docker registry
# Usage: ./scripts/build-and-push-local.sh [--registry REGISTRY] [--skip-platform] [--skip-backend] [--skip-frontend]

set -euo pipefail

# Default values
REGISTRY="${DOCKER_REGISTRY:-localhost:5000}"
BUILD_PLATFORM=true
BUILD_BACKEND=true
BUILD_FRONTEND=true
BUILD_MODE="${BUILD_MODE:-release}"
PARALLEL_BUILDS="${PARALLEL_BUILDS:-4}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --registry)
            REGISTRY="$2"
            shift 2
            ;;
        --skip-platform)
            BUILD_PLATFORM=false
            shift
            ;;
        --skip-backend)
            BUILD_BACKEND=false
            shift
            ;;
        --skip-frontend)
            BUILD_FRONTEND=false
            shift
            ;;
        --debug)
            BUILD_MODE="debug"
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --registry REGISTRY    Docker registry to push to (default: localhost:5000)"
            echo "  --skip-platform        Skip building platform services"
            echo "  --skip-backend         Skip building backend services"
            echo "  --skip-frontend        Skip building frontend services"
            echo "  --debug                Build in debug mode instead of release"
            echo "  --help                 Show this help message"
            echo ""
            echo "Environment variables:"
            echo "  DOCKER_REGISTRY        Same as --registry flag"
            echo "  BUILD_MODE             Build mode: release or debug (default: release)"
            echo "  PARALLEL_BUILDS        Number of parallel builds (default: 4)"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Print configuration
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}SI Docker Build Configuration${NC}"
echo -e "${BLUE}========================================${NC}"
echo -e "Registry:        ${GREEN}${REGISTRY}${NC}"
echo -e "Build Mode:      ${GREEN}${BUILD_MODE}${NC}"
echo -e "Build Platform:  ${GREEN}${BUILD_PLATFORM}${NC}"
echo -e "Build Backend:   ${GREEN}${BUILD_BACKEND}${NC}"
echo -e "Build Frontend:  ${GREEN}${BUILD_FRONTEND}${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Check if local registry is running
check_registry() {
    if [[ "${REGISTRY}" == localhost:* ]] || [[ "${REGISTRY}" == 127.0.0.1:* ]]; then
        echo -e "${YELLOW}Checking if local Docker registry is running...${NC}"
        if ! curl -s "http://${REGISTRY}/v2/" > /dev/null; then
            echo -e "${YELLOW}Local registry not found. Starting local registry...${NC}"
            docker run -d -p 5000:5000 --name registry --restart=always registry:2
            sleep 2
        else
            echo -e "${GREEN}Local registry is running${NC}"
        fi
    fi
}

# Build and push a Docker image
build_and_push() {
    local name=$1
    local dockerfile=$2
    local build_args=${3:-}
    local context=${4:-.}

    local image="${REGISTRY}/si-${name}:latest"

    echo -e "\n${BLUE}========================================${NC}"
    echo -e "${BLUE}Building: ${name}${NC}"
    echo -e "${BLUE}========================================${NC}"

    # Build command
    local build_cmd="docker build -t ${image} -f ${dockerfile} ${build_args} ${context}"

    echo -e "${YELLOW}Running: ${build_cmd}${NC}"

    if eval "${build_cmd}"; then
        echo -e "${GREEN}✓ Built ${name}${NC}"

        echo -e "${YELLOW}Pushing ${image}...${NC}"
        if docker push "${image}"; then
            echo -e "${GREEN}✓ Pushed ${name}${NC}"
        else
            echo -e "${RED}✗ Failed to push ${name}${NC}"
            return 1
        fi
    else
        echo -e "${RED}✗ Failed to build ${name}${NC}"
        return 1
    fi
}

# Build Rust service
build_rust_service() {
    local service=$1
    build_and_push "${service}" "Dockerfile.rust" "--build-arg SERVICE=${service} --build-arg BUILD_MODE=${BUILD_MODE}" "."
}

# Change to repo root
cd "$(dirname "$0")/.."

# Check registry
check_registry

# ============================================================================
# Platform Services
# ============================================================================
if [ "${BUILD_PLATFORM}" = true ]; then
    echo -e "\n${BLUE}========================================${NC}"
    echo -e "${BLUE}Building Platform Services${NC}"
    echo -e "${BLUE}========================================${NC}"

    # Build from existing Dockerfiles in component/
    build_and_push "postgres" "component/postgres/Dockerfile" "--build-arg BASE_VERSION=14.5-bullseye" "component/postgres"
    build_and_push "pgbouncer" "component/pgbouncer/Dockerfile" "--build-arg BASE_VERSION=1.22.1-p0" "component/pgbouncer"
    build_and_push "nats" "component/nats/Dockerfile" "--build-arg BASE_VERSION=2.11.4" "component/nats"
    build_and_push "spicedb" "component/spicedb/Dockerfile" "" "component/spicedb"
    build_and_push "otelcol" "component/otelcol/Dockerfile" "" "component/otelcol"
    build_and_push "versitygw" "component/versitygw/Dockerfile" "" "component/versitygw"
fi

# ============================================================================
# Backend Services (Rust)
# ============================================================================
if [ "${BUILD_BACKEND}" = true ]; then
    echo -e "\n${BLUE}========================================${NC}"
    echo -e "${BLUE}Building Backend Services (Rust)${NC}"
    echo -e "${BLUE}========================================${NC}"

    # Define Rust services
    RUST_SERVICES=(
        "sdf"
        "veritech"
        "rebaser"
        "pinga"
        "edda"
        "forklift"
        "module-index"
    )

    # Build Rust services
    for service in "${RUST_SERVICES[@]}"; do
        build_rust_service "${service}"
    done

    # Build auth-api (Node.js)
    echo -e "\n${BLUE}Building auth-api (Node.js)${NC}"
    build_and_push "auth-api" "bin/auth-api/Dockerfile" "" "."
fi

# ============================================================================
# Frontend Services
# ============================================================================
if [ "${BUILD_FRONTEND}" = true ]; then
    echo -e "\n${BLUE}========================================${NC}"
    echo -e "${BLUE}Building Frontend Services${NC}"
    echo -e "${BLUE}========================================${NC}"

    build_and_push "web" "Dockerfile.web" "" "."
    build_and_push "auth-portal" "Dockerfile.auth-portal" "" "."
    build_and_push "docs" "Dockerfile.docs" "" "."
fi

# ============================================================================
# Summary
# ============================================================================
echo -e "\n${GREEN}========================================${NC}"
echo -e "${GREEN}Build Complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo -e "All images have been pushed to ${GREEN}${REGISTRY}${NC}"
echo ""
echo -e "To start the services, run:"
echo -e "  ${YELLOW}docker-compose -f docker-compose.selfhost.yml up -d${NC}"
echo ""
echo -e "To view logs:"
echo -e "  ${YELLOW}docker-compose -f docker-compose.selfhost.yml logs -f${NC}"
echo ""
echo -e "To stop services:"
echo -e "  ${YELLOW}docker-compose -f docker-compose.selfhost.yml down${NC}"
echo ""
