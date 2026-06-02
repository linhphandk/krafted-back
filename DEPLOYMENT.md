# Production Deployment on AWS Lightsail

This project is now ready for production deployment on AWS Lightsail.

## Production Docker Compose

The production-ready docker-compose file is located at `docker-compose.prod.yml`.

## Environment Variables

For production, use the `.env.prod` file or set environment variables directly:

```
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
DATABASE_URL=postgres://krafted:krafted@db:5432/krafted
DATABASE_POOL_SIZE=4
JWT_SECRET=your-super-secret-jwt-key-here
JWT_EXPIRY_MINUTES=15
RUST_LOG=info
```

## Deployment Steps

1. Create an Ubuntu 22.04 LTS instance on AWS Lightsail
2. SSH into your instance:
   ```
   ssh ubuntu@your-instance-ip
   ```

3. Install Docker and Docker Compose:
   ```
   sudo apt update
   sudo apt install docker.io docker-compose -y
   ```

4. Create a directory for your application:
   ```
   mkdir -p /opt/krafted
   cd /opt/krafted
   ```

5. Copy your application files to the instance:
   - `docker-compose.prod.yml`
   - `.env.prod` (with your JWT_SECRET)
   - `Dockerfile`
   - All source files and migrations

6. Set proper permissions:
   ```
   chmod +x deploy.sh
   ```

7. Set the JWT_SECRET as an environment variable on your host system:
   ```
   export JWT_SECRET=your-super-secret-jwt-key-here
   ```

8. Run the deployment:
   ```
   docker compose -f docker-compose.prod.yml up -d
   ```

## AWS Lightsail Configuration

1. Configure firewall rules to allow:
   - Port 3000 (for your application API)
   - Port 5432 (for database access - internal only)

2. For a reverse proxy with HTTPS, consider setting up Nginx or Traefik

3. Consider setting up a domain name and SSL certificate for production use

## Testing Your Deployment

Once deployed, you can test your API:
```
curl http://your-lightsail-ip:3000/health
```

The application should respond with a 200 OK status.

## Note on Adminer

Adminer was removed from the production setup as it's only needed for development purposes. For database management in production, use direct database connections or tools like `psql` from the command line.

## Environment Variables in Docker

The application loads environment variables directly from the container's environment using the `envy` crate. When running in Docker:
- Environment variables are passed through the `environment` section in `docker-compose.prod.yml`
- The `.env` file is NOT loaded in Docker containers
- This ensures consistent behavior between local development and production

## Local Development

For local development, the application automatically loads environment variables from the `.env` file using `dotenvy`:

1. Copy `.env.example` to `.env`:
   ```
   cp .env.example .env
   ```

2. Edit `.env` with your local settings

3. Run locally:
   ```
   cargo run
   ```

For production Docker deployments, environment variables are passed directly to the container via Docker Compose, not from .env files.