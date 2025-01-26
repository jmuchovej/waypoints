{ pkgs, lib, config, inputs, ... }:

{

  # https://devenv.sh/packages/
  packages = with pkgs; [ just git ];

  # https://devenv.sh/languages/
  languages.dart = {
    enable  = true;
  };
  languages.javascript = {
    enable      = true;
    bun.enable  = true;
  };

  # https://devenv.sh/processes/
  # processes.cargo-watch.exec = "cargo-watch";

  # https://devenv.sh/services/
  # services.postgres.enable = true;

  # https://devenv.sh/scripts/

  enterShell = ''
  '';

  # https://devenv.sh/tasks/
  # tasks = {
  # };

  # https://devenv.sh/tests/
  enterTest = ''
  '';

  # https://devenv.sh/pre-commit-hooks/
  # pre-commit.hooks.shellcheck.enable = true;

  # See full reference at https://devenv.sh/reference/options/
}
