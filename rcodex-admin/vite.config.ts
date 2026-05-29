import path from "path"
import { defineConfig } from "vite"
import react from "@vitejs/plugin-react"

export default defineConfig({
  plugins: [react()],
  base: "./",  // Use relative paths for assets
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  server: {
    port: 3000,
    proxy: {
      "/admin": {
        target: "http://localhost:8788",
        changeOrigin: true,
      },
    },
  },
  preview: {
    port: 3003,
    proxy: {
      "/admin": {
        target: "http://localhost:8788",
        changeOrigin: true,
      },
    },
  },
})
