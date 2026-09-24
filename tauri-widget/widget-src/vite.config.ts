import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// 小组件独立构建：产物输出到 ../widget-dist，供 Tauri 内嵌
// base: './' 使资源路径为相对路径，适配 tauri:// 协议加载
export default defineConfig({
  plugins: [react()],
  base: './',
  build: {
    outDir: '../widget-dist',
    emptyOutDir: true,
    target: 'es2020',
    cssCodeSplit: false,
    rollupOptions: {
      output: {
        entryFileNames: 'widget.js',
        assetFileNames: 'widget.[ext]',
      },
    },
  },
});
