#!/bin/bash

# Production deployment script for krafted-back

echo "Starting production deployment..."

# Build and start the application
echo "Building and starting services..."
docker compose -f docker-compose.prod.yml up -d

# Check if services are running
echo "Checking service status..."
docker compose -f docker-compose.prod.yml ps

echo "Deployment complete!"
echo "Application should be available at http://your-lightsail-ip:3000"