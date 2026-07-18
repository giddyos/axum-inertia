import { defineConfig } from "vite";

export default defineConfig({
  build: {
    manifest: true,
    outDir: "dist",
    emptyOutDir: true,
    rollupOptions: {
      input: "src/main.ts",
    },
  },
});
