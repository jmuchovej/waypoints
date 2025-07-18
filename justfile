# Tempo Development Tasks
# Run `just` to see all available commands

# Default task - show all available commands
default:
  @just --list

# Build runner tasks
build:
  dart run build_runner build --delete-conflicting-outputs

watch:
  dart run build_runner watch --dev

# Generate app flavors
flavorizr:
  dart run flutter_flavorizr

# Dependency management
update-deps: update-deps-flutter update-deps-macos update-deps-ios

update-deps-flutter:
  flutter pub upgrade

update-deps-macos:
  cd macos && pod repo update

update-deps-ios:
  cd ios && pod repo update

# Clean Flutter build artifacts
clean:
  flutter clean

install:
  flutter pub get

# Run the app in different flavors
run-dev:
  flutter run --flavor development

run-staging:
  flutter run --flavor staging

run-prod:
  flutter run --flavor production

# Build for different platforms
build-ios flavor="production":
  flutter build ios --flavor {{flavor}}

build-android flavor="production":
  flutter build apk --flavor {{flavor}}

build-macos flavor="production":
  flutter build macos --flavor {{flavor}}

# Testing
test:
  flutter test

test-unit:
  flutter test test/unit/

test-widget:
  flutter test test/widgets/

test-integration:
  flutter test test/integration/

# Code analysis and formatting
analyze:
  flutter analyze

format:
  dart format ./lib/

format-check:
  dart format --set-exit-if-changed .

# Linting with ruff (for Python files if any)
lint-python:
  ruff check .

# CocoaPods management
install-pods:
  cd ios && pod install
  cd macos && pod install

# Android SDK management
check-android:
  flutter doctor -v | grep -A 20 "Android toolchain"

# Development environment checks
doctor:
  flutter doctor -v

# FVM-specific commands (assuming fvm is installed)
fvm-install:
  fvm install 3.32.0

fvm-use:
  fvm use 3.32.0

# Database tasks
db-reset:
  rm -f tempo-flux.db
  echo "Database reset"

# Git hooks
install-hooks:
  echo "#!/bin/sh\njust format-check && just analyze" > .git/hooks/pre-commit
  chmod +x .git/hooks/pre-commit

# Combined development workflow commands
dev: watch
  @echo "Starting build watcher..."

# Quick checks before committing
pre-commit: format-check analyze test

# Full CI simulation
ci: clean
  just format-check
  just analyze
  just test
  just build-android
  just build-ios

# Setup commands for new developers
setup:
  just fvm-install
  just fvm-use
  just update-deps
  just install-pods
  just doctor
