import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Tauri loads the built dist; relative paths are needed for tauri://localhost.
export default defineConfig({
  plugins: [svelte()],
  base: './',
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: { target: 'es2022', sourcemap: false, chunkSizeWarningLimit: 600 },
});
