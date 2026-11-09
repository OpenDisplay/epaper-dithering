#!/usr/bin/env bash
#
# Build EpaperDithering.xcframework from the iOS FFI wrapper crate.
#
# Requires macOS with Xcode + Rust (rustup) and the Apple iOS targets:
#   rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
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

echo "==> Building release staticlib for each target"
for t in "${DEVICE_TARGET}" "${SIM_TARGETS[@]}"; do
  cargo build --release --manifest-path "${CRATE_DIR}/Cargo.toml" --target "$t"
done

# Strip debug (-S) and local (-x) symbols from each static archive. The exported globals
# (ed_dither, ed_dither_options_default*, ed_abi_version) are retained for linking; this ~cuts the vendored size by a third.
echo "==> Stripping static archives"
for t in "${DEVICE_TARGET}" "${SIM_TARGETS[@]}"; do
  strip -S -x "${CRATE_DIR}/target/${t}/release/${LIB_NAME}"
done

# Xcode's strip must understand the LLVM version rustc emits. An older Xcode (e.g. 26.x with
# rustc 1.99 / LLVM 23) exits 0 but leaves std/compiler_builtins unstripped (~14 MB instead of
# ~2.4 MB for the device slice). rust's own llvm-strip is no substitute: its output trips ld's
# "LSDA in __eh_frame" assertion. So use a current Xcode, and say so loudly when it isn't.
DEVICE_LIB="${CRATE_DIR}/target/${DEVICE_TARGET}/release/${LIB_NAME}"
DEVICE_SIZE=$(stat -f%z "${DEVICE_LIB}")
if [ "${DEVICE_SIZE}" -gt 6000000 ]; then
  echo "warning: ${DEVICE_TARGET} archive is ${DEVICE_SIZE} bytes after strip (expected ~2.4 MB)." >&2
  echo "warning: $(xcodebuild -version | head -1) may be too old for $(rustc --version); use a newer Xcode." >&2
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
