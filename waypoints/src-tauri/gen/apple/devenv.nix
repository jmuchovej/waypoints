{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:
{
  profiles.ios.module =
    { config, ... }:
    {
      languages.swift.enable = true;

      # Cross-compilation: cc-rs checks SDKROOT before calling the compiler.
      # Nix sets it to its macOS SDK, which cc-rs rejects for iOS and then
      # falls back to xcrun (which also fails under Nix). Fix: unset SDKROOT
      # (Nix's clang has its sysroot baked in) and set DEVELOPER_DIR so xcrun
      # can resolve any SDK.
      enterShell = ''
        if [ -d "/Applications/Xcode.app/Contents/Developer" ]; then
          unset SDKROOT
          export DEVELOPER_DIR="/Applications/Xcode.app/Contents/Developer"
          # Nix's clang bakes in -mmacos-version-min which conflicts with iOS
          # targets. Use /usr/bin/clang (Xcode's) for iOS cross-compilation.
          export CC_aarch64_apple_ios="/usr/bin/clang"
          export CXX_aarch64_apple_ios="/usr/bin/clang++"
          export CARGO_TARGET_AARCH64_APPLE_IOS_LINKER="/usr/bin/clang"
          export CC_aarch64_apple_ios_sim="/usr/bin/clang"
          export CXX_aarch64_apple_ios_sim="/usr/bin/clang++"
          export CARGO_TARGET_AARCH64_APPLE_IOS_SIM_LINKER="/usr/bin/clang"
          # Nix's Swift wrapper lacks swift-build (SPM). Prepend Xcode's
          # toolchain so swift-rs can compile Swift packages.
          export PATH="$DEVELOPER_DIR/Toolchains/XcodeDefault.xctoolchain/usr/bin:$PATH"
        fi
      '';
    };
}
