# Dev server — reads live source via the docker-compose volume mount.
# This means no static build is needed; Vite picks up all file changes.
FROM node:22-alpine

WORKDIR /app

# Copy package manifests
COPY package*.json ./

# Fast install with npm cache mount, skipping audit & fund checks
RUN --mount=type=cache,target=/root/.npm \
    npm install --include=optional --no-audit --no-fund

# Copy configuration files into image layer
COPY postcss.config.js tailwind.config.js svelte.config.js vite.config.js index.html ./

EXPOSE 5173

# Health check using wget built into Alpine
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget -q -O- http://localhost:5173 || exit 1

CMD ["npm", "run", "dev"]
