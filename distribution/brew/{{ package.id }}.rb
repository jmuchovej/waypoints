# SPDX-FileCopyrightText: 2026 John Muchovej
# SPDX-License-Identifier: AGPL-3.0-or-later

cask "{{ package.id }}" do
  arch arm: "arm64", intel: "amd64"
  os macos: "macos", linux: "linux"

  version "{{ release.version }}"
  sha256 arm: "{{ artifact.macos_arm64.dmg.sha256 }}",
         x86_64_linux: "{{ artifact.linux_amd64.AppImage.sha256 }}"

  url "https://waypoints.so/api/v1/releases/{{ release.channel }}/{{ release.series }}/#{os}/#{arch}/waypoints-v#{version}.#{on_system_conditional(macos: "dmg", linux: "AppImage")}"
  name "{{ flavor.product_name }}"
  desc "Local-first knowledge management"
  homepage "https://waypoints.so/"

  livecheck do
    url "https://waypoints.so/api/v1/releases/{{ release.channel }}/current"
    strategy :json do |json|
      json["tag"]&.delete_prefix("v")
    end
  end

  auto_updates true

  on_macos do
    depends_on arch: :arm64
    depends_on macos: ">= :sequoia"

    app "{{ flavor.product_name }}.app"

    uninstall quit: "{{ flavor.identifier }}"

    zap trash: [
      "~/Library/Application Support/{{ flavor.identifier }}",
      "~/Library/Caches/{{ flavor.identifier }}",
    ]
  end

  on_linux do
    depends_on arch: :x86_64

    app_image "{{ artifact.linux_amd64.AppImage.file_name }}", target: "{{ flavor.executable }}"

    zap trash: [
      "~/.cache/{{ flavor.identifier }}",
      "~/.config/{{ flavor.identifier }}",
      "~/.local/share/{{ flavor.identifier }}",
    ]
  end
end
