import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
// @ts-expect-error sin el paquete @types/node
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// Opciones para `tauri dev` / `tauri build`: Tauri espera un puerto fijo, y los errores
// de Rust no se deben borrar de la consola
export default defineConfig(() => ({
  plugins: [sveltekit()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
