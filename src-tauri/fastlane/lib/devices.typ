// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

// The devices a capture is framed in. Every number is in the frame image's own
// pixels, as `res/alpha-key.nu` prints them: the image's size, the screen rect
// inside it, and its corner radius. `scale` is frame pixels per capture pixel.
// The images are licensed renders kept out of history — see `res/.gitkeep`.
#let devices = (
  iphone-17-pro: (
    image: "/fastlane/res/apple-iphone-17-pro.png",
    size: (1317, 2716),
    screen: (x: 58, y: 49, width: 1202, height: 2619),
    radius: 222,
    scale: 1,
  ),
  pixel-10-pro: (
    image: "/fastlane/res/google-pixel-10-pro.png",
    size: (2781, 5854),
    screen: (x: 121, y: 120, width: 2519, height: 5615),
    radius: 338,
    scale: 2,
  ),
)
