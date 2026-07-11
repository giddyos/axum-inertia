import inertia from '@inertiajs/vite'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [vue(), inertia()],
  build: {
    manifest: true,
    rollupOptions: {
      input: 'src/main.ts',
    },
  },
})
