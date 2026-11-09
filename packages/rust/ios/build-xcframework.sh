#!/usr/bin/env bash
#
# Build EpaperDithering.xcframework from the iOS FFI wrapper crate.
#
# Requires macOS with Xcode + Rust (rustup), the Apple iOS targets and llvm-tools:
#   rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
#   rustup component add llvm-tools
#
# Output: ./EpaperDithering.xcframework  (device arm64 + simulator arm64/x86_64 fat)
#
# This is the ONLY step that needs Xcode. Everything else in the repo (core, python,
# javascript, and `cargo test` of this crate on the host) builds without it.
set -euo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB_NAME="libepaper_dithering_ios.a"
OUT_DIR="${CRATE_DIR}/EpaperDithering.xcframework"
HEADERS_SRC="${CRATE_DIR}/include"
BUILD="${CRATE_DIR}/.build"

DEVICE_TARGET="aarch64-apple-ios"
SIM_TARGETS=("aarch64-apple-ios-sim" "x86_64-apple-ios")

echo "==> Checking toolchain"
command -v cargo >/dev/null || { echo "error: cargo not found (install rustup)"; exit 1; }
command -v xcodebuild >/dev/null || { echo "error: xcodebuild not found (install Xcode)"; exit 1; }
for t in "${DEVICE_TARGET}" "${SIM_TARGETS[@]}"; do
  rustup target list --installed | grep -qx "$t" || {
    echo "error: missing rust target '$t' — run: rustup target add $t"; exit 1; }
done
# llvm-objcopy from rustc's own LLVM (see "Removing embedded bitcode" below).
LLVM_OBJCOPY="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin/llvm-objcopy"
[ -x "${LLVM_OBJCOPY}" ] || {
  echo "error: llvm-objcopy not found — run: rustup component add llvm-tools"; exit 1; }

echo "==> Building release staticlib for each target"
for t in "${DEVICE_TARGET}" "${SIM_TARGETS[@]}"; do
  cargo build --release --manifest-path "${CRATE_DIR}/Cargo.toml" --target "$t"
done

# Removing embedded bitcode: the prebuilt std/core/compiler_builtins objects carry their LLVM
# bitcode in a __LLVM,__bitcode section (~5 MB for std alone). Xcode 27's strip drops it, but
# Xcode 26's (all GitHub's macOS runners have) keeps it, which left the device slice at ~12 MB
# instead of ~2.4 MB. Apple's bitcode_strip can't take a static archive, so remove just that
# section with llvm-objcopy, which reads rustc's objects whatever the Xcode version. (Use it
# for nothing else: llvm-strip's broader rewrite makes ld assert on "LSDA in __eh_frame".)
echo "==> Removing embedded bitcode"
for t in "${DEVICE_TARGET}" "${SIM_TARGETS[@]}"; do
  "${LLVM_OBJCOPY}" --remove-section=__LLVM,__bitcode "${CRATE_DIR}/target/${t}/release/${LIB_NAME}"
done

# Strip debug (-S) and local (-x) symbols from each static archive. The exported globals
# (ed_dither, ed_dither_options_default*, ed_abi_version) are retained for linking.
echo "==> Stripping static archives"
for t in "${DEVICE_TARGET}" "${SIM_TARGETS[@]}"; do
  strip -S -x "${CRATE_DIR}/target/${t}/release/${LIB_NAME}"
done

# Guard: an archive this big means bitcode or debug info survived (expected ~2.4 MB).
DEVICE_SIZE=$(stat -f%z "${CRATE_DIR}/target/${DEVICE_TARGET}/release/${LIB_NAME}")
if [ "${DEVICE_SIZE}" -gt 6000000 ]; then
  echo "error: ${DEVICE_TARGET} archive is ${DEVICE_SIZE} bytes after stripping (expected ~2.4 MB)" >&2
  exit 1
fi

# The ios crate is workspace-excluded, so it is its own workspace root and cargo writes
# its target dir local to the crate.
TARGET_ROOT="${CRATE_DIR}/target"

echo "==> Assembling simulator fat archive (lipo)"
rm -rf "${BUILD}"; mkdir -p "${BUILD}/sim" "${BUILD}/device" "${BUILD}/headers"
SIM_INPUTS=()
for t in "${SIM_TARGETS[@]}"; do
  SIM_INPUTS+=("${TARGET_ROOT}/${t}/release/${LIB_NAME}")
done
lipo -create "${SIM_INPUTS[@]}" -output "${BUILD}/sim/${LIB_NAME}"
cp "${TARGET_ROOT}/${DEVICE_TARGET}/release/${LIB_NAME}" "${BUILD}/device/${LIB_NAME}"

echo "==> Staging headers (+ modulemap)"
cp "${HEADERS_SRC}/epaper_dithering.h" "${BUILD}/headers/"
cp "${HEADERS_SRC}/module.modulemap"   "${BUILD}/headers/"

echo "==> Creating XCFramework"
rm -rf "${OUT_DIR}"
xcodebuild -create-xcframework \
  -library "${BUILD}/device/${LIB_NAME}" -headers "${BUILD}/headers" \
  -library "${BUILD}/sim/${LIB_NAME}"    -headers "${BUILD}/headers" \
  -output "${OUT_DIR}"

echo "==> Done: ${OUT_DIR}"
echo "    Device:    arm64 (${DEVICE_TARGET})"
echo "    Simulator: arm64 + x86_64 (fat)"
