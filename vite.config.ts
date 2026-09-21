import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    strictPort: true,
    host: '127.0.0.1',
    port: 1420,
  },
  // P2-15: 不暴露 TAURI_* 前缀——签名密钥等经进程环境注入，TAURI_ 前缀会把
  // 它们整批带进 import.meta.env 进而进 bundle。src/ 不消费任何 TAURI_* env。
  envPrefix: 'VITE_',
  build: {
    target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'safari13',
    minify: process.env.TAURI_ENV_DEBUG ? false : 'esbuild',
    sourcemap: Boolean(process.env.TAURI_ENV_DEBUG),
  },
  test: {
    environment: 'jsdom',
    globals: true,
    css: true,
  },
})

