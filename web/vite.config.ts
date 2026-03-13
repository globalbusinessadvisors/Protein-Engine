import path from "path";
import { defineConfig } from "vite";
import wasm from "vite-plugin-wasm";

export default defineConfig({
  plugins: [wasm()],
  resolve: {
    alias: {
      "pe-wasm": path.resolve(__dirname, "pkg/pe_wasm"),
    },
  },
  build: {
    target: "esnext",
  },
});
