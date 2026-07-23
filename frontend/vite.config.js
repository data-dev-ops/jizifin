import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  server: {
    host: true,          // bind to 0.0.0.0 inside container
    port: 5173,
    strictPort: true,
    allowedHosts: true,
    // Forward /api and /ws to the backend container or local backend.
    proxy: {
      '/api': {
        target: process.env.VITE_BACKEND_URL || 'http://backend:8000',
        changeOrigin: true,
        ws: true,
        rewrite: (path) => path.replace(/^\/api/, ''),
        configure: (proxy) => {
          proxy.on('error', (err, _req, _res) => {
            // Fallback for running Vite outside Docker directly on host machine
            if (err.code === 'ECONNREFUSED' && !process.env.VITE_BACKEND_URL) {
              // Ignore or handle
            }
          });
        }
      },
      '/ws': {
        target: process.env.VITE_WS_BACKEND_URL || 'ws://backend:8000',
        ws: true
      },
    },
  },
});
