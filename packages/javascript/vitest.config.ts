import { defineConfig, type Plugin } from 'vitest/config';
import fs from 'fs';

/** Inline .wasm files as Uint8Array — mirrors esbuild binary loader used by tsup. */
const wasmBinaryPlugin: Plugin = {
  name: 'wasm-binary',
  enforce: 'pre',  // Must run before Vite's built-in WASM ESM handler
  load(id) {
    if (!id.endsWith('.wasm')) return null;
    const bytes = Array.from(fs.readFileSync(id));
    return { code: `export default new Uint8Array([${bytes.join(',')}]);`, map: null };
  },
};

export default defineConfig({
  plugins: [wasmBinaryPlugin],
  test: {
    globals: true,
    environment: 'node',
    coverage: {
      provider: 'v8',
      reporter: ['text', 'lcov'],
      include: ['src/**/*.ts'],
      // Generated wasm-bindgen output and type declarations: not code under test, and
      // vitest ≥4's coverage cannot parse ambient declarations (`export const x: T;`).
      exclude: ['src/wasm-core/**', 'src/**/*.d.ts'],
    },
  },
});
