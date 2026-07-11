import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

export default defineConfig(({ isSsrBuild }) => ({
  plugins: [svelte({ prebundleSvelteLibraries: true }), inertia({ ssr: { host: '127.0.0.1', port: 13714 } })],
  build: {
    outDir: isSsrBuild ? 'dist/ssr' : '../public/build',
    emptyOutDir: true,
    manifest: !isSsrBuild,
    rollupOptions: isSsrBuild ? {} : {
      input: 'src/app.js',
    },
  },
}))
import inertia from '@inertiajs/vite'
