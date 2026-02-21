#!/bin/bash
set -e

# Get the directory where the script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT_DIR="$SCRIPT_DIR/../../../../"

# Configuration
CRATE_NAME="localgpt_mobile"
RELEASE_MODE="--release"
TARGET_DIR="$ROOT_DIR/target"
ANDROID_LIB_DIR="$SCRIPT_DIR/../LocalGPTLib"

# Targets
ARM64_TARGET="aarch64-linux-android"
X86_64_TARGET="x86_64-linux-android"

echo "Building for Android ARM64 ($ARM64_TARGET)..."
cargo ndk -t arm64-v8a build -p localgpt-mobile $RELEASE_MODE

echo "Building for Android x86_64 ($X86_64_TARGET)..."
cargo ndk -t x86_64 build -p localgpt-mobile $RELEASE_MODE

# Output directories
mkdir -p "$ANDROID_LIB_DIR/src/main/jniLibs/arm64-v8a"
mkdir -p "$ANDROID_LIB_DIR/src/main/jniLibs/x86_64"
mkdir -p "$ANDROID_LIB_DIR/src/main/java"

# Copy shared libraries
cp "$TARGET_DIR/$ARM64_TARGET/release/lib$CRATE_NAME.so" "$ANDROID_LIB_DIR/src/main/jniLibs/arm64-v8a/"
cp "$TARGET_DIR/$X86_64_TARGET/release/lib$CRATE_NAME.so" "$ANDROID_LIB_DIR/src/main/jniLibs/x86_64/"

# Generate Kotlin Bindings
echo "Generating UniFFI Bindings for Kotlin..."
# Use the compiled Android library to generate bindings
# Note: UniFFI needs a library path to find the metadata
LIBRARY_PATH="$TARGET_DIR/$ARM64_TARGET/release/lib$CRATE_NAME.so"

cargo run --bin uniffi-bindgen -p localgpt-mobile -- generate \
    --library "$LIBRARY_PATH" \
    --language kotlin \
    --out-dir "$ANDROID_LIB_DIR/src/main/java"

echo "Build complete! Android library updated at $ANDROID_LIB_DIR"
