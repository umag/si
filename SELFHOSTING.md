# SI Self-Hosting Guide

This guide explains how to build and deploy System Initiative (SI) services for self-hosting using Docker and Docker Compose.

## Prerequisites

- Docker Engine 20.10+ and Docker Compose v2
- At least 8GB RAM available for Docker
- 50GB+ free disk space
- Linux, macOS, or Windows with WSL2

## Architecture

The self-hosted deployment consists of three main groups of services:

### Platform Services
- **PostgreSQL**: Primary database for all services
- **NATS**: Message queue for inter-service communication
- **SpiceDB**: Authorization and permissions service
- **OpenTelemetry Collector**: Telemetry and observability
- **VersityGW**: S3-compatible object storage

### Backend Services
- **SDF**: Core API service (GraphQL/REST)
- **Veritech**: Infrastructure execution engine
- **Rebaser**: Conflict resolution and merging
- **Pinga**: Health monitoring
- **Edda**: Change history and audit service
- **Forklift**: Data migration service
- **Module Index**: Module catalog service
- **Auth API**: Authentication API (Node.js)

### Frontend Services
- **Web**: Main Vue.js SPA
- **Auth Portal**: Authentication/workspace portal
- **Docs**: Documentation site

## Quick Start

### 1. Configure Environment Variables

Copy the example environment file and customize it:

```bash
cp .env.selfhost.example .env
```

Edit `.env` and set secure passwords and secrets:
- `POSTGRES_PASSWORD`
- `SPICEDB_PRESHARED_KEY`
- `ZED_KEYRING_PASSWORD`
- `JWT_SECRET`

**Authentication:** By default, LOCAL_AUTH_MODE=true is configured, which bypasses Auth0 and uses local authentication. This is recommended for self-hosting. If you need Auth0 for production SSO, set `LOCAL_AUTH_MODE=false` and configure Auth0 credentials.

### 2. Start Local Docker Registry

If you don't have a Docker registry, the build script will automatically start one:

```bash
docker run -d -p 5000:5000 --name registry --restart=always registry:2
```

### 3. Build and Push Images

Build all services and push to local registry:

```bash
./scripts/build-and-push-local.sh
```

This will:
- Check if local registry is running
- Build all platform services
- Build all backend services (Rust + Node.js)
- Build all frontend services
- Push images to `localhost:5000`

**Build Options:**

```bash
# Build only backend services
./scripts/build-and-push-local.sh --skip-platform --skip-frontend

# Build only platform services
./scripts/build-and-push-local.sh --skip-backend --skip-frontend

# Build in debug mode (faster builds, larger images)
./scripts/build-and-push-local.sh --debug

# Use a custom registry
./scripts/build-and-push-local.sh --registry my-registry.example.com:5000
```

### 4. Start Services

Start all services with Docker Compose:

```bash
docker-compose -f docker-compose.selfhost.yml up -d
```

### 5. Verify Services

Check that all services are running:

```bash
docker-compose -f docker-compose.selfhost.yml ps
```

View logs:

```bash
# All services
docker-compose -f docker-compose.selfhost.yml logs -f

# Specific service
docker-compose -f docker-compose.selfhost.yml logs -f sdf
```

### 6. Access Services

Once running, you can access:

- **Web UI**: http://localhost:8080
- **Auth Portal**: http://localhost:9000
- **API (SDF)**: http://localhost:5156/api/
- **Documentation**: http://localhost:5173
- **NATS Dashboard**: http://localhost:8222

## Service Dependencies

The services have the following startup dependencies:

```
postgres → rebaser, veritech, sdf, module-index, auth-api
nats → rebaser, edda, forklift, pinga, veritech, sdf
spicedb → sdf
otelcol → rebaser, edda, forklift, pinga, veritech, sdf, module-index
veritech → pinga
pinga → sdf
rebaser, edda, forklift → sdf
sdf → web
auth-api → auth-portal
```

## Persistent Data

The following volumes are created for persistent data:

- `postgres_data`: PostgreSQL database files
- `nats_data`: NATS JetStream data
- `versity_data`: S3-compatible object storage

To back up data:

```bash
# Backup PostgreSQL
docker-compose -f docker-compose.selfhost.yml exec postgres pg_dumpall -U si > backup.sql

# Backup volumes
docker run --rm -v si_postgres_data:/data -v $(pwd):/backup alpine tar czf /backup/postgres_backup.tar.gz /data
```

## Troubleshooting

### Services fail to start

1. Check logs for the failing service:
   ```bash
   docker-compose -f docker-compose.selfhost.yml logs <service-name>
   ```

2. Verify environment variables are set correctly in `.env`

3. Ensure dependent services are healthy:
   ```bash
   docker-compose -f docker-compose.selfhost.yml ps
   ```

### PostgreSQL connection errors

- Verify `POSTGRES_PASSWORD` matches in all service configurations
- Check if PostgreSQL is healthy: `docker-compose -f docker-compose.selfhost.yml exec postgres pg_isready`

### Out of memory errors

- Increase Docker memory limit (Docker Desktop: Settings → Resources)
- Reduce `PARALLEL_BUILDS` in build script
- Consider scaling down non-essential services

### Build failures

1. Ensure you have enough disk space (50GB+)
2. Check Buck2 cache: The Rust builds use Buck2 which may require RBE credentials
3. Try building in debug mode: `--debug` flag (faster but larger images)

## Authentication Options

SI supports two authentication modes:

### Local Auth Mode (Default for Self-Hosting)

Set `LOCAL_AUTH_MODE=true` in your `.env` file (this is the default).

Benefits:
- No external dependencies
- Works offline
- Instant setup
- Auto-creates a local development workspace
- Bypasses email verification and onboarding

Limitations:
- Single hardcoded user (`dev@systeminit.local`)
- No SSO/OAuth integration
- Suitable for development, testing, and small deployments

### Auth0 Mode (For Production SSO)

Set `LOCAL_AUTH_MODE=false` and configure Auth0 credentials:
```bash
LOCAL_AUTH_MODE=false
AUTH0_DOMAIN=your-tenant.auth0.com
AUTH0_CLIENT_ID=your_client_id
AUTH0_CLIENT_SECRET=your_client_secret
```

Benefits:
- Multi-user support
- SSO/OAuth integration
- Email verification
- Production-ready user management

Use Auth0 mode for production deployments requiring multiple users, SSO, or enterprise authentication.

## Production Considerations

For production deployments:

1. **Authentication**: Consider using Auth0 mode (`LOCAL_AUTH_MODE=false`) for multi-user support and SSO
2. **Use external PostgreSQL**: Replace the postgres service with a managed database
3. **Configure TLS**: Set up SSL/TLS certificates for all services
4. **Use secrets management**: Don't store secrets in `.env`, use Docker secrets or vault
5. **Scale services**: Use Docker Swarm or Kubernetes for multi-instance deployments
6. **Set up monitoring**: Re-enable Grafana, Prometheus, Loki for observability
7. **Configure backups**: Set up automated backups for PostgreSQL and volumes
8. **Use a reverse proxy**: Put nginx or Traefik in front of services
9. **Harden security**: Follow security best practices (non-root users, network policies, etc.)

## Updating Services

To update to a new version:

1. Pull latest code
2. Rebuild images: `./scripts/build-and-push-local.sh`
3. Restart services: `docker-compose -f docker-compose.selfhost.yml up -d`

## Stopping Services

```bash
# Stop services but keep data
docker-compose -f docker-compose.selfhost.yml down

# Stop services and remove volumes (WARNING: deletes all data)
docker-compose -f docker-compose.selfhost.yml down -v
```

## Advanced Configuration

### Custom Build Arguments

The Rust Dockerfile accepts build arguments:

- `SERVICE`: Name of the Rust service to build (required)
- `BUILD_MODE`: `release` (default) or `debug`
- `RUST_VERSION`: Rust version (default: 1.86)
- `DEBIAN_VERSION`: Debian version (default: bookworm-slim)

Example:

```bash
docker build -f Dockerfile.rust \
  --build-arg SERVICE=sdf \
  --build-arg BUILD_MODE=release \
  -t localhost:5000/si-sdf:latest \
  .
```

### Using a Remote Registry

To push to a remote registry:

```bash
export DOCKER_REGISTRY=registry.example.com
docker login registry.example.com
./scripts/build-and-push-local.sh
```

Then update `docker-compose.selfhost.yml` to reference the remote registry.

## Support

For issues and questions:
- GitHub Issues: https://github.com/systeminit/si/issues
- Documentation: https://docs.systeminit.com

## License

System Initiative is licensed under the Apache License 2.0. See LICENSE file for details.
