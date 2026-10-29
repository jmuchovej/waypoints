#!/usr/bin/env nu
# Keys the screen out of a Mockuuups device-frame export.
#
#   alpha-key.nu exports/<device>.png [--out <path>] [--ring 3]
#
# Mockuuups fills the screen with opaque black, so the frame is exported over a
# white screen background and the white is removed here, in two passes:
#
# 1. A flood fill from the screen's centre finds the pure-white interior. Never
#    key by colour — the bezel highlights are also pure white.
# 2. The screen edge is anti-aliased against that white, so on a thin ring just
#    outside the interior each pixel is treated as bezel colour over a white
#    matte and un-matted: alpha = 1 - min(r,g,b), colour = (c - min) / (1 - min).
#    Without this the edge keeps a pale halo.
#
# Prints the screen rect and corner radius, which is what `lib/devices.typ`
# records for the device.

def main [
  export: path       # the un-keyed export, e.g. exports/apple-iphone-17-pro.png
  --out (-o): path   # the keyed frame; default: the export's name beside this script
  --ring: int = 3    # width of the edge band to un-matte, in pixels
]: nothing -> record {
  let out = $out | default ($env.FILE_PWD | path join ($export | path basename))
  let tmp = mktemp -d
  let t = $tmp | path join ($export | path parse | get stem)
  let mask = $"($t)-mask.png"
  let ring = $"($t)-ring.png"

  let size = ^magick identify -format "%w %h" $export | split row " " | into int
  let w = $size.0
  let h = $size.1

  # The screen mask (white = screen), and the ring just outside it.
  (^magick $export -alpha off -fuzz 0% -fill "#FF0000"
    -draw $"color ($w // 2),($h // 2) floodfill"
    -fill black +opaque "#FF0000" -fill white -opaque "#FF0000" $mask)
  (^magick $mask -morphology Dilate $"Disk:($ring)"
    "(" $mask -negate ")" -compose Multiply -composite $ring)

  # Remove the white matte everywhere, then keep it only on the ring.
  ^magick $export -alpha extract $"($t)-alpha.png"
  ^magick $export -alpha off -separate -evaluate-sequence Min $"($t)-min.png"
  ^magick $"($t)-min.png" -negate $"($t)-alpha-unmatted.png"
  ^magick $export -alpha off $"($t)-min.png" -fx "(u-v)/max(1-v,0.004)" $"($t)-rgb-unmatted.png"
  ^magick $export -alpha off $"($t)-rgb-unmatted.png" $ring -composite $"($t)-rgb.png"
  (^magick $"($t)-alpha.png"
    "(" $"($t)-alpha.png" $"($t)-alpha-unmatted.png" -compose Darken -composite ")" $ring -composite
    "(" $mask -negate ")" -compose Multiply -composite $"($t)-alpha-out.png")
  ^magick $"($t)-rgb.png" $"($t)-alpha-out.png" -compose CopyOpacity -composite $out

  # The mask's bounding box is the screen rect; where the top row of that box
  # first turns white is the corner radius.
  let screen = ^magick $mask -format "%@" info: | parse "{w}x{h}+{x}+{y}" | first | into int w h x y
  let radius = (^magick $mask -crop $"($screen.w)x1+($screen.x)+($screen.y)" +repage -format "%@" info:
    | parse "{w}x{h}+{x}+{y}" | first | get x | into int)

  rm -r $tmp

  {
    frame: $out
    export: $"($w)x($h)"
    screen: $"($screen.w)x($screen.h) at ($screen.x),($screen.y)"
    radius: $radius
  }
}
