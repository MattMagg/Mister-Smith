# MisterSmith Monitoring UI - Docker Setup Guide

## Overview

This Docker setup provides a complete OpenTelemetry + Jaeger telemetry stack for the MisterSmith monitoring UI application.

## Architecture

```
Frontend App (React) → OpenTelemetry Collector → Jaeger Backend
                                ↓
                           Prometheus (metrics)
```

## Services

- **jaeger**: All-in-one Jaeger container (UI + backend)
- **otel-collector**: OpenTelemetry Collector with OTLP receiver
- **prometheus**: Metrics storage and monitoring
- **mistersmith-ui**: React development server (optional)

## Quick Start

### Prerequisites

- Docker Desktop or Docker CLI
- Node.js 20+
- npm

### 1. Start the Telemetry Stack

```bash
./scripts/start-telemetry.sh
```

This will:
- Start Jaeger, OpenTelemetry Collector, and Prometheus
- Wait for services to become healthy
- Display service URLs and status

### 2. Install Dependencies and Start UI

```bash
npm install
npm run dev
```

### 3. Access Services

- **Jaeger UI**: http://localhost:16686
- **Prometheus**: http://localhost:9090
- **UI Application**: http://localhost:5173
- **OTel Collector Health**: http://localhost:13133

### 4. Stop the Stack

```bash
./scripts/stop-telemetry.sh
```

## Configuration

### Environment Variables

The UI application automatically detects Docker environment and configures endpoints:

- `VITE_METRICS_ENDPOINT`: OpenTelemetry metrics endpoint
- `VITE_TRACES_ENDPOINT`: OpenTelemetry traces endpoint
- `VITE_JAEGER_UI_URL`: Jaeger UI URL
- `VITE_PROMETHEUS_URL`: Prometheus URL

### Docker Environment Detection

The application automatically detects Docker environment by:
- Checking if hostname is `mistersmith-ui`
- Checking `VITE_DOCKER_ENV` environment variable

## Telemetry Pipeline

1. **Frontend Application** → Generates metrics and traces
2. **OpenTelemetry Collector** → Receives and processes telemetry data
3. **Jaeger** → Stores and displays traces
4. **Prometheus** → Stores and queries metrics

## Endpoints

### OpenTelemetry Collector

- **OTLP gRPC**: `http://localhost:4317`
- **OTLP HTTP**: `http://localhost:4318`
- **Health Check**: `http://localhost:13133`
- **Prometheus Metrics**: `http://localhost:8889`

### Jaeger

- **UI**: `http://localhost:16686`
- **HTTP Collector**: `http://localhost:14268`
- **gRPC Collector**: `http://localhost:14250`

### Prometheus

- **UI**: `http://localhost:9090`
- **API**: `http://localhost:9090/api/v1/`

## Troubleshooting

### Check Service Health

```bash
docker-compose ps
```

### View Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f jaeger
docker-compose logs -f otel-collector
docker-compose logs -f prometheus
```

### Restart Services

```bash
./scripts/stop-telemetry.sh
./scripts/start-telemetry.sh
```

### Clean Reset

```bash
./scripts/stop-telemetry.sh --clean
docker-compose down -v  # Remove volumes
./scripts/start-telemetry.sh
```

## Development

### Building UI Container

```bash
docker-compose up mistersmith-ui --build
```

### Running Tests

```bash
npm test
```

### Debugging Telemetry

1. Check console logs in browser for OpenTelemetry initialization
2. Visit `http://localhost:13133` for collector health
3. Check Jaeger UI for traces
4. Check Prometheus for metrics

## Configuration Files

- `docker-compose.yml`: Main Docker Compose configuration
- `otel-collector-config.yaml`: OpenTelemetry Collector configuration
- `prometheus.yml`: Prometheus scraping configuration
- `Dockerfile.dev`: Development container for UI

## Network

All services use the `mistersmith-telemetry` bridge network for internal communication.

## Performance

- **Metrics Export**: Every 10 seconds
- **Trace Batching**: 512 spans per batch
- **Prometheus Scraping**: Every 15 seconds
- **Memory Limits**: 256MB for collector

## Security

- All services run on localhost
- No authentication required for development
- CORS enabled for UI development
- No sensitive data in logs

## Production Considerations

For production deployment:

1. Enable authentication on Jaeger and Prometheus
2. Use external storage for Jaeger (not memory)
3. Configure proper resource limits
4. Set up log rotation
5. Use TLS for all communications
6. Configure proper RBAC