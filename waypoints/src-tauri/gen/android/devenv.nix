{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:
{
  profiles.android.module =
    { config, ... }:
    {
      languages.kotlin.enable = true;
      android = {
        enable = true;
        ndk.enable = true;
        platforms.version = [
          "36"
          "34"
        ];
        buildTools.version = [
          "35.0.0"
          "34.0.0"
        ];
      };

      # Cross-compilation: point cc-rs at the NDK toolchain instead of
      # Nix's host-only clang wrapper.
      enterShell = ''
        NDK_VERSION=$(ls "$ANDROID_HOME/ndk" | head -1)
        export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/$NDK_VERSION"
        NDK_TOOLCHAIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin"
        export CC_aarch64_linux_android="$NDK_TOOLCHAIN/aarch64-linux-android28-clang"
        export CXX_aarch64_linux_android="$NDK_TOOLCHAIN/aarch64-linux-android28-clang++"
        export AR_aarch64_linux_android="$NDK_TOOLCHAIN/llvm-ar"
        export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK_TOOLCHAIN/aarch64-linux-android28-clang"
      '';
    };
}
