{ pkgs, lib, config, ... }:

{
  # https://github.com/cachix/devenv/issues/1174#issuecomment-2088381294
  # stdenv = pkgs.stdenvNoCC;
  # https://devenv.sh/basics/
  #! Needs to be set during `enterShell` because something else sets `DEVELOPER_DIR` 👀
  env.DEVELOPER_DIR = "/Applications/Xcode.app/Contents/Developer";
  # env.SDKROOT = "/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk";
  env.CHROME_EXECUTABLE = "${pkgs.arc-browser}/Applications/Arc.app/Contents/MacOS/Arc";

  # https://devenv.sh/packages/
  packages = (with pkgs; [
    git curl wget cmake
    protobuf protoc-gen-go protoc-gen-go-grpc protoc-gen-entgrpc
    turso-cli
    rustup
    biome
    bacon
    # just  # Needs to be >1.37.0
    cargo-binstall
    # cargo-tauri  # install `tauri` to `cargo tauri`
    # Apple Dev Tools
    cocoapods
    xcodes xcode-install xcbuild xcpretty xcbeautify
    darwin.xcode
    ios-deploy
  ]);

  # https://devenv.sh/integrations/android/
  android = {
    enable                = true;
    # Implicitly enables `languages.dart`!
    flutter.enable        = true;
    android-studio.enable = false;
  };
  languages.java = {
    enable      = true;
    jdk.package = lib.mkDefault pkgs.jdk17;
  };

  languages.go = {
    enable = true;
  };

  # https://devenv.sh/languages/
  languages.javascript = {
    enable              = true;
    yarn.enable         = true;
    bun.enable          = true;
    bun.install.enable  = true;
  };
  languages.typescript.enable = true;
  # languages.rust              = {
  #   enable  = true;
  #   channel = "stable";
  #   targets = [
  #     "aarch64-apple-darwin"  "aarch64-apple-ios"
  #     "x86_64-apple-darwin"   "x86_64-apple-ios"
  #     "aarch64-unknown-linux-gnu"
  #     "x86_64-unknown-linux-gnu"
  #   ];
  # };

  # https://devenv.sh/processes/
  # processes.api-server.exec = ''
  #   cargo watch -q -c -w packages/server/src -x run
  # '';

  # https://devenv.sh/services/
  # services.postgres = {
  #   enable            = true;
  #   initialDatabases  = [
  #     { name = "chopper"; user = "chopper"; pass = "chopper"; }
  #   ];
  #   # https://devenv.sh/supported-services/postgres/#servicespostgresextensions
  #   extensions        = null;
  # };

  # https://devenv.sh/scripts/

  enterShell = ''
    # rustup default ${config.languages.rust.channel}
    # cargo install cargo-smart-release
    export DEVELOPER_DIR="/Applications/Xcode.app/Contents/Developer"
  '';

  # https://devenv.sh/tasks/
  # tasks = {
  #   "myproj:setup".exec = "mytool build";
  #   "devenv:enterShell".after = [ "myproj:setup" ];
  # };

  # https://devenv.sh/tests/
  enterTest = ''
    echo "Running tests"
    git --version | grep --color=auto "${pkgs.git.version}"
  '';

  # https://devenv.sh/pre-commit-hooks/
  # pre-commit.hooks.shellcheck.enable = true;

  # See full reference at https://devenv.sh/reference/options/
}
