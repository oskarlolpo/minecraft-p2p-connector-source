import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  root: "src",
  publicDir: false,
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 3000,
    strictPort: true,
  },
  build: {
    outDir: "../dist",
    target: "es2022",
    emptyOutDir: true,
  },
});
