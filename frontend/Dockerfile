# Dev server — reads live source via the docker-compose volume mount.
# This means no static build is needed; Vite picks up all file changes.
FROM node:22-alpine

WORKDIR /app

# Install dependencies so they are baked into the image layer.
# The node_modules anonymous volume in compose keeps them even if the
# host ./frontend directory is mounted over /app.
COPY package*.json ./
RUN npm ci

EXPOSE 5173

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget -q -O- http://localhost:5173 || exit 1

CMD ["npm", "run", "dev"]
