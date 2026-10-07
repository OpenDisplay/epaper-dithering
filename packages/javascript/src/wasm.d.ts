/** esbuild binary loader inlines .wasm files as Uint8Array */
declare module '*.wasm' {
  const bytes: Uint8Array;
  export default bytes;
}
