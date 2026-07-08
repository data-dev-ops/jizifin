import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  server: {
    host: true,          // bind to 0.0.0.0 inside container
    port: 5173,
    strictPort: true,
    allowedHosts: ['jizifin.duckdns.org', 'localhost', '134.209.137.40'],
  },
});
