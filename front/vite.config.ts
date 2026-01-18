import path from 'node:path'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    host: '0.0.0.0',
    port: 8118,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:8119',
        changeOrigin: true,
      },
    },
  },
})
