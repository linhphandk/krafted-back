# Development and Production Environment Variables

## Local Development

For local development, the application uses the `.env` file automatically via `dotenvy`:

1. Copy `.env.example` to `.env`:
   ```
   cp .env.example .env
   ```

2. Edit `.env` with your local settings:
   ```
   SERVER_HOST=127.0.0.1
   SERVER_PORT=3000
   DATABASE_URL=postgres://krafted:krafted@localhost:5432/krafted
   DATABASE_POOL_SIZE=4
   JWT_SECRET=your-local-secret-key
   JWT_EXPIRY_MINUTES=15
   RUST_LOG=info
   ```

3. Run locally:
   ```
   cargo run
   ```

## Production Deployment

For production Docker deployments, environment variables are passed through Docker Compose:

1. Use `.env.prod` for production settings:
   ```
   SERVER_HOST=0.0.0.0
   SERVER_PORT=3000
   DATABASE_URL=postgres://krafted:krafted@db:5432/krafted
   DATABASE_POOL_SIZE=4
   JWT_SECRET=your-super-secret-jwt-key-here
   JWT_EXPIRY_MINUTES=15
   RUST_LOG=info
   ```

2. Set the JWT_SECRET as an environment variable on your host system:
   ```
   export JWT_SECRET=your-super-secret-jwt-key-here
   ```

3. Run with Docker Compose:
   ```
   docker compose -f docker-compose.prod.yml up -d
   ```

## Security Note

The `.env` file is not copied into the Docker image for security reasons. Production environment variables should be set through Docker Compose environment variables or system environment variables, never stored in the image.