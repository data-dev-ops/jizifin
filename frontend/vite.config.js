import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  server: {
    host: true,          // bind to 0.0.0.0 inside container
    port: 5173,
    strictPort: true,
    allowedHosts: ['jizifin.duckdns.org', 'localhost', '134.209.137.40'],
    // Forward /api and /ws to the local backend when running outside Docker.
    // In production, Caddy handles this routing before requests reach Vite,
    // so this proxy block is simply never exercised there.
    proxy: {
      // Mirrors Caddy's `handle_path /api/*`: strip the /api prefix before
      // forwarding to the backend, so /api/users → backend:8000/users.
      '/api': { target: 'http://localhost:8000', changeOrigin: true, rewrite: (path) => path.replace(/^\/api/, '') },
      '/ws':  { target: 'ws://localhost:8000',   ws: true },
    },
  },
});
