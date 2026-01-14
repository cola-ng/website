# Docker Setup for Colang Website

This document describes how to build and run the Colang website using Docker containers.

## Project Structure

The project consists of two main parts:
- **Backend**: Rust application using the Salvo web framework
- **Frontend**: React application built with Vite

## Building Docker Images

### Backend
```bash
cd backend
docker build -t colang-backend .
```

### Frontend
```bash
cd frontend
docker build -t colang-frontend .
```

## Running with Docker Compose

To run the entire application stack:

```bash
docker-compose up -d
```

This will start:
- Backend service on port 6019
- Frontend service on port 80
- PostgreSQL database on port 5432

## GitHub Actions Workflows

The project includes several GitHub Actions workflows for continuous integration and deployment:

### 1. CI Build (`.github/workflows/ci.yml`)
- Runs tests for both backend and frontend
- Builds Docker images for both services
- Performs code quality checks

### 2. Docker Build (`.github/workflows/docker-build.yml`)
- Builds and pushes Docker images to GitHub Container Registry (GHCR)
- Builds for multiple architectures (AMD64 and ARM64)
- Creates different tags based on branch, PR, or SHA

### 3. Release (`.github/workflows/release.yml`)
- Builds and publishes Docker images when a GitHub release is created
- Tags images with release version and latest

## GitHub Container Registry

Docker images are published to GHCR at:
- Backend: `ghcr.io/{username}/{repository}-backend`
- Frontend: `ghcr.io/{username}/{repository}-frontend`

## Environment Variables

The backend service expects the following environment variables:
- `DATABASE_URL`: PostgreSQL database connection string
- `JWT_SECRET`: Secret key for JWT token signing
- `BIND_ADDR`: Address to bind the server to (default: 0.0.0.0:6019)

## Deployment

To deploy this application to a container service:
1. Ensure your container service can pull images from GHCR
2. Update the `docker-compose.yml` file with the appropriate image tags
3. Deploy using your preferred container orchestration platform