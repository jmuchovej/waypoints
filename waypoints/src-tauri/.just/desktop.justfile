# Desktop aliasing for macOS, Windows, and Linux

mod macos "macOS.justfile"
mod windows "windows.justfile"
mod linux "linux.justfile"

[private]
default:
    @just --list

dev *ARGS:
    @just {{ os() }}::dev {{ ARGS }}

dev-preview *ARGS:
    @just {{ os() }}::dev-preview {{ ARGS }}

dev-stable *ARGS:
    @just {{ os() }}::dev-stable {{ ARGS }}

build *ARGS:
    @just {{ os() }}::build {{ ARGS }}

build-preview *ARGS:
    @just {{ os() }}::build-preview {{ ARGS }}

build-nightly *ARGS:
    @just {{ os() }}::build-nightly {{ ARGS }}
