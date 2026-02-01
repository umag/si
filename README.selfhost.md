# System Initiative - Self-Hosting

This directory contains everything needed to self-host System Initiative using Docker.

## 🚀 Quick Start

Run the interactive setup:

```bash
./scripts/quickstart.sh
```

This will:
1. Check prerequisites
2. Generate secure passwords
3. Configure local authentication (no Auth0 required)
4. Build all Docker images
5. Start all services

## 📁 What's Included

- **Dockerfiles** for all services (Rust, Node.js, Vue.js)
- **docker-compose.selfhost.yml** for orchestration
- **Build scripts** for automated image building
- **Makefile** with convenient commands
- **Complete documentation** in SELFHOSTING.md

## 🏗️ Services

### Platform Layer
- PostgreSQL (database)
- NATS (message queue)
- SpiceDB (authorization)
- OpenTelemetry (observability)
- VersityGW (S3-compatible storage)

### Backend Services
- SDF (core API)
- Veritech (execution engine)
- Rebaser, Pinga, Edda, Forklift, Module-Index
- Auth API

### Frontend Services
- Web UI (Vue.js)
- Auth Portal
- Documentation

## 📖 Documentation

- **SELFHOSTING.md** - Complete guide with architecture, troubleshooting, and production tips
- **SELFHOST_SUMMARY.md** - Quick overview of created files
- **.env.selfhost.example** - Configuration template

## 🛠️ Common Commands

Using the Makefile:

```bash
# Build all services
make -f Makefile.selfhost build

# Start services
make -f Makefile.selfhost start

# View logs
make -f Makefile.selfhost logs

# Stop services
make -f Makefile.selfhost stop

# Check status
make -f Makefile.selfhost status

# Backup database
make -f Makefile.selfhost backup-db

# See all commands
make -f Makefile.selfhost help
```

Or use Docker Compose directly:

```bash
docker-compose -f docker-compose.selfhost.yml up -d
docker-compose -f docker-compose.selfhost.yml logs -f
docker-compose -f docker-compose.selfhost.yml down
```

## 🌐 Access Points

After starting services:

- **Web UI**: http://localhost:8080
- **Auth Portal**: http://localhost:9000
- **API**: http://localhost:5156/api/
- **Documentation**: http://localhost:5173
- **NATS Dashboard**: http://localhost:8222

## ⚙️ Configuration

1. Copy environment template:
   ```bash
   cp .env.selfhost.example .env
   ```

2. Edit `.env` and set:
   - Database passwords
   - JWT secrets
   - Other configuration

**Authentication:** Local auth mode (`LOCAL_AUTH_MODE=true`) is enabled by default - no Auth0 setup required. For production deployments with multiple users and SSO, you can switch to Auth0 mode. See SELFHOSTING.md for details.

## 🔨 Building Images

Build all services:

```bash
./scripts/build-and-push-local.sh
```

Build options:

```bash
# Build only backend
./scripts/build-and-push-local.sh --skip-platform --skip-frontend

# Build in debug mode (faster)
./scripts/build-and-push-local.sh --debug

# Use custom registry
./scripts/build-and-push-local.sh --registry my-registry.example.com:5000
```

## 📊 System Requirements

- **RAM**: 8GB minimum, 16GB recommended
- **Disk**: 50GB free space minimum
- **CPU**: 4+ cores recommended
- **OS**: Linux, macOS, or Windows with WSL2

## 🔍 Troubleshooting

See SELFHOSTING.md for detailed troubleshooting.

Quick checks:

```bash
# Check service status
docker-compose -f docker-compose.selfhost.yml ps

# View service logs
docker-compose -f docker-compose.selfhost.yml logs <service-name>

# Check service health
make -f Makefile.selfhost health

# Restart a service
docker-compose -f docker-compose.selfhost.yml restart <service-name>
```

## 🔐 Security Notes

- Change all default passwords in `.env`
- Use secrets management in production (Docker secrets, Vault)
- Configure TLS/SSL for production deployments
- Review security settings in SELFHOSTING.md

## 📦 Persistent Data

Data is stored in Docker volumes:
- `postgres_data` - PostgreSQL database
- `nats_data` - NATS JetStream data
- `versity_data` - Object storage

Backup regularly:

```bash
make -f Makefile.selfhost backup-db
```

## 🚢 Production Deployment

For production use:

1. Review SELFHOSTING.md production checklist
2. Use external PostgreSQL (managed database)
3. Configure proper TLS certificates
4. Set up monitoring and alerting
5. Implement backup and disaster recovery
6. Use Docker Swarm or Kubernetes for HA

## 📚 Additional Resources

- Main documentation: https://docs.systeminit.com
- GitHub repository: https://github.com/systeminit/si
- Issues: https://github.com/systeminit/si/issues

## 📝 License

Apache License 2.0 - See LICENSE file for details.
