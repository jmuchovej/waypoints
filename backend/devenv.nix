{ pkgs, lib, config, ... }:

{
  # https://devenv.sh/basics/

  # https://devenv.sh/packages/
  packages = (with pkgs; [
    git curl wget cmake
    protobuf
    protoc-gen-go protoc-gen-go-grpc protoc-gen-entgrpc protoc-gen-dart
    bacon
    just
  ]);

  # https://devenv.sh/languages/
  languages.go.enable = true;

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
  '';

  # https://devenv.sh/tasks/
  # tasks = {
  #   "myproj:setup".exec = "mytool build";
  #   "devenv:enterShell".after = [ "myproj:setup" ];
  # };

  # https://devenv.sh/tests/
  enterTest = ''
  '';

  # https://devenv.sh/pre-commit-hooks/
  # pre-commit.hooks.shellcheck.enable = true;

  # See full reference at https://devenv.sh/reference/options/
}
