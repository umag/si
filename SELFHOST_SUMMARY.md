# SI Self-Hosting Setup - Summary

This document summarizes the files created for self-hosting System Initiative.

## Created Files

### Dockerfiles

1. **Dockerfile.rust** - Generic multi-stage Dockerfile for all Rust services
   - Uses Buck2 for building
   - Supports both release and debug modes
   - Creates minimal runtime images based on Debian slim

2. **Dockerfile.web** - Vue.js web frontend
   - Multi-stage build with pnpm
   - Nginx-based runtime with SPA routing
   - Includes API and WebSocket proxying

3. **Dockerfile.auth-portal** - Authentication portal
   - Node.js/Vue build
   - Nginx-based runtime for static SPA

4. **Dockerfile.docs** - VitePress documentation
   - Builds documentation site
   - Nginx-based static hosting

### Docker Compose

5. **docker-compose.selfhost.yml** - Main compose file for selfhosting
   - Platform services: PostgreSQL, NATS, SpiceDB, OpenTelemetry, VersityGW
   - Backend services: SDF, Veritech, Rebaser, Pinga, Edda, Forklift, Module-Index, Auth-API
   - Frontend services: Web, Auth-Portal, Docs
   - Configured service dependencies and health checks
   - Persistent volumes for data
   - Excludes monitoring/telemetry services as requested

6. **docker-compose.override.yml.example** - Template for custom overrides

### Scripts

7. **scripts/build-and-push-local.sh** - Build and push script
8. **scripts/quickstart.sh** - Interactive setup script

### Configuration & Documentation

9. **.env.selfhost.example** - Environment variables template
10. **Makefile.selfhost** - Convenient make targets
11. **nginx-spa.conf** - Enhanced nginx configuration (updated)
12. **.dockerignore** - Docker build context optimization
13. **SELFHOSTING.md** - Complete self-hosting guide

## Quick Start

```bash
./scripts/quickstart.sh
```

Or see SELFHOSTING.md for detailed instructions.
