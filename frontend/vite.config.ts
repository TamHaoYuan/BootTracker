/// <reference types="vitest" />
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'node:path';

// 后端默认端口（与 server/config.py 的 PORT 一致）
const BACKEND_PORT = Number(process.env.BOOTTRACKER_PORT || 18792);

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, 'src'),
    },
  },
  // WebEngine 加载 http://127.0.0.1:{port}/，base 用相对路径，避免 file:// 资源路径问题
  base: './',
  build: {
    // 产物输出到项目根的 dist-static/，供 PyInstaller 直接打包
    outDir: path.resolve(__dirname, '../dist-static'),
    emptyOutDir: true,
    sourcemap: false,
    target: 'es2020',
    rollupOptions: {
      output: {
        // 第三方库拆分，便于按需加载
        manualChunks: {
          react: ['react', 'react-dom', 'react-router-dom'],
          antd: ['antd', '@ant-design/icons'],
        },
      },
    },
  },
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      // 前端独立开发时，/api 代理到 Python 后端
      '/api': {
        target: `http://127.0.0.1:${BACKEND_PORT}`,
        changeOrigin: true,
      },
    },
  },
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: './tests/setup.ts',
    css: false,
  },
});
