# Development Environment

For local development, you can use the `.env` file which will be automatically loaded by dotenvy.

However, when running in Docker containers, the application relies on environment variables passed through the Docker Compose configuration rather than the .env file.

The .env file is not used in production Docker containers, only for local development.