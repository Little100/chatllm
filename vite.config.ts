import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue(), tailwindcss()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },

  // Tauri 开发服务器配置
  clearScreen: false,
  server: {
    host: 'localhost',
    port: 1420,
    strictPort: true,
  },
  envPrefix: ['VITE_'],
})
