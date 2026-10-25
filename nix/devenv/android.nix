# Owns Android toolchain assembly: the SDK composed from android-nixpkgs,
# system-image expansion per platform version, the ANDROID_*/LD_LIBRARY_PATH/
# PATH environment, and the gradle-facing generated files (`local.properties`,
# `libs.versions.toml`). Versions are data under `waypoints.android.*`; AGP,
# build-tools, and kotlin stay in lockstep through the generated catalog.
#
# The emulator and its system images are opt-in (`waypoints.android.emulator`,
# switched on by the `emulator` devenv profile): they run an AVD and nothing
# else, and at roughly 20 GB they are what a CI runner cannot hold. Building
# the app needs only the platforms, build-tools and NDK.
{
  lib,
  config,
  pkgs,
  inputs,
  ...
}:
let
  inherit (lib) mkOption types;

  cfg = config.waypoints.android;

  # Map a version (e.g. "35.0.0") to its android-nixpkgs attribute suffix ("35-0-0").
  dash-version = lib.replaceStrings [ "." ] [ "-" ];

  # The NDK prebuilt host toolchain directory each supported system builds from.
  ndk-hosts = {
    # NOTE: Android ships arm64-darwin-compatible tools under x86_64-darwin
    aarch64-darwin = "darwin-x86_64";
    x86_64-darwin = "darwin-x86_64";
    # NOTE: Android doesn't support building from arm64-linux
    x86_64-linux = "linux-x86_64";
  };

  host-system = pkgs.stdenv.hostPlatform.system;

  ndk-host =
    ndk-hosts.${host-system} or (throw (
      "No Android NDK host toolchain for ${host-system}; supported: "
      + lib.concatStringsSep ", " (lib.attrNames ndk-hosts)
    ));

  # only the `compileSdk` needs to be installed. the `minSdk` and `targetSdk`
  # just declare compatibility targets. all other installed platforms should go
  # in `platforms.extra`.
  platform-versions = lib.unique ([ cfg.sdk.compile.version ] ++ cfg.platforms.extra);

  # ps16k (16 KB page-size) system images only exist for API >= 35.
  system-images =
    sdk-pkgs:
    let
      base-variants = [
        "google-apis"
        "google-apis-playstore"
      ];
      variants =
        v: base-variants ++ lib.optionals (lib.versionAtLeast v "35") (map (s: "${s}-ps16k") base-variants);
    in
    lib.concatMap (
      v: map (s: sdk-pkgs."system-images-android-${dash-version v}-${s}-arm64-v8a") (variants v)
    ) platform-versions;

  android-sdk = inputs.android-nixpkgs.sdk.${host-system} (
    sdk-pkgs:
    (
      [
        sdk-pkgs.cmdline-tools-22-0
        sdk-pkgs.platform-tools
        sdk-pkgs."build-tools-${dash-version cfg.build-tools.version}"
        sdk-pkgs."ndk-${dash-version cfg.ndk.version}"
      ]
      ++ map (v: sdk-pkgs."platforms-android-${dash-version v}") platform-versions
      ++ lib.optionals cfg.emulator.enable ([ sdk-pkgs.emulator ] ++ system-images sdk-pkgs)
    )
  );
  android-home = "${android-sdk}/share/android-sdk";
in
{
  options.waypoints.android = {
    agp.version = mkOption {
      type = types.str;
      description = "Android Gradle Plugin version pinned into the generated version catalog.";
    };
    build-tools.version = mkOption {
      type = types.str;
      description = "Android build-tools version (SDK component and version catalog).";
    };
    sdk = {
      min.version = mkOption {
        type = types.str;
        readOnly = true;
        default =
          let
            tauri = fromTOML (builtins.readFile "${config.devenv.root}/src-tauri/Tauri.toml");
          in
          toString tauri.bundle.android.minSdkVersion;
        description = "Oldest supported API level. Read from the Tauri config, which owns it — `tauri-cli` deserializes `bundle.android.minSdkVersion` itself, and that struct is `deny_unknown_fields`, so it cannot live anywhere else.";
      };
      target.version = mkOption {
        type = types.str;
        description = "API level whose behaviors the app opts into (`targetSdk`).";
      };
      compile.version = mkOption {
        type = types.str;
        description = "API level the app compiles against (`compileSdk`).";
      };
    };
    platforms.extra = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = "Platform versions to install beyond the three SDK roles — emulator images and older test targets.";
    };
    emulator.enable = mkOption {
      type = types.bool;
      default = false;
      description = "Install the emulator and a system image per platform version and variant. Only an AVD needs these; builds do not.";
    };
    ndk.version = mkOption {
      type = types.str;
      description = "Android NDK version (SDK component and ANDROID_NDK_ROOT).";
    };
    project-path = mkOption {
      type = types.str;
      description = "Repo-relative path of the generated Tauri Android project; gradle-facing files land here.";
    };
  };

  config = {
    env = {
      ANDROID_HOME = android-home;
      ANDROID_NDK_ROOT = "${config.env.ANDROID_HOME}/ndk/${cfg.ndk.version}";
      ANDROID_USER_HOME = "${config.env.DEVENV_STATE}/android";
      ANDROID_AVD_HOME = "${config.env.ANDROID_USER_HOME}/avd";
      LD_LIBRARY_PATH = lib.concatStringsSep ":" [
        "$LD_LIBRARY_PATH"
        (pkgs.lib.makeLibraryPath [
          pkgs.vulkan-loader
          pkgs.libGL
        ])
        "${config.env.ANDROID_HOME}/build-tools/${cfg.build-tools.version}/lib64/"
        "${config.env.ANDROID_NDK_ROOT}/toolchains/llvm/prebuilt/${ndk-host}/lib/"
        "$LD_LIBRARY_PATH"
      ];
      # env.PATH merges as one string, so this module owns the whole
      # definition; the non-Android `.bin` prefix rides along.
      PATH = lib.concatStringsSep ":" [
        "$PATH"
        "${config.env.ANDROID_HOME}/tools"
        "${config.env.ANDROID_HOME}/tools/bin"
        "${config.env.ANDROID_HOME}/platform-tools"
        "${config.env.ANDROID_HOME}/cmdline-tools/bin"
        "${config.env.ANDROID_HOME}/emulator"
      ];
    };

    packages = [ android-sdk ];

    files = {
      "${cfg.project-path}/local.properties".text = ''
        # This file was automatically generated by nix-shell.
        sdk.dir=${config.env.ANDROID_HOME}
        android.aapt2FromMavenOverride=${config.env.ANDROID_HOME}/build-tools/${cfg.build-tools.version}/aapt2
      '';
      "${cfg.project-path}/gradle/libs.versions.toml" = {
        copyMode = "copy";
        toml = {
          versions = {
            kotlin = config.waypoints.mise.tools.kotlin.package.version;
            buildTools = cfg.build-tools.version;
            agp = cfg.agp.version;
            ndk = cfg.ndk.version;
            minSdk = cfg.sdk.min.version;
            targetSdk = cfg.sdk.target.version;
            compileSdk = cfg.sdk.compile.version;
          };
        };
      };
      # The wrapper itself is gitignored, so Tauri runs the `gradle` on PATH —
      # the one devenv provides and mise projects. The properties file is
      # generated from that same package so it cannot pin a different version
      # than the one actually running, and so a Gradle bump is one move.
      "${cfg.project-path}/gradle/wrapper/gradle-wrapper.properties" = {
        copyMode = "copy";
        text = ''
          distributionBase=GRADLE_USER_HOME
          distributionUrl=https\://services.gradle.org/distributions/gradle-${config.waypoints.mise.tools.gradle.package.version}-bin.zip
          distributionPath=wrapper/dists
          zipStorePath=wrapper/dists
          zipStoreBase=GRADLE_USER_HOME
        '';
      };
    };
  };
}
