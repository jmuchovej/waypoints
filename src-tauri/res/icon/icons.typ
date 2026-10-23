// Bundle entry point: renders every icon for the selected flavors into a
// staging tree that mirrors `src-tauri/`, plus `manifest.json` telling
// `cargo xtask icongen` what to pack and where it may prune. Run from the app
// directory as root:
//
//   typst compile --root src-tauri --format bundle --ppi 72 \
//     src-tauri/res/icon/icons.typ <staging> [--input flavors=stable,nightly]
//
// `members/` holds pack inputs only; nothing under it is placed.

#import "/res/icon/lib.typ": icon

#let sizes = toml("/res/sizes.toml")
#let all-flavors = sizes.short.keys()
#let flavors = sys.inputs.at("flavors", default: all-flavors.join(",")).split(",")
#for f in flavors {
  assert(f in all-flavors, message: "unknown flavor `" + f + "`; expected one of " + all-flavors.join(", "))
}

#let expand(template, flavor) = template.replace("{flavor}", flavor).replace("{short}", sizes.short.at(flavor))

#let png(path, px, body) = document(path, page(width: px * 1pt, height: px * 1pt, margin: 0pt, fill: none, body))

// `#rrggbb[aa]` → `#aarrggbb`, the order Android color resources use.
#let argb(c) = {
  let h = c.to-hex().slice(1)
  let (rgb, a) = if h.len() == 8 { (h.slice(0, 6), h.slice(6)) } else { (h, "ff") }
  "#" + a + rgb
}

#let manifest = (owned: (), icns: (), ico: (), webp: (), place: ())

#for flavor in flavors {
  import "/res/palettes/" + flavor + ".typ": fills, app
  // (fills, background, tint); the default is the app icon itself.
  let render(platform, px, style: (fills.dark, app.background-inner, app.background-outer)) = {
    let (theme, background, tint) = style
    icon(platform, px, theme, background: background, tint: tint)
  }
  let members = "members/" + flavor + "/"

  // --- desktop ---------------------------------------------------------------

  for e in sizes.linux {
    let path = expand(e.file, flavor)
    png(path, e.px, render("linux", e.px))
    manifest.place.push(path)
  }

  let icns = expand(sizes.macOS.file, flavor)
  for px in sizes.macOS.members.map(m => m.px).dedup() {
    png(members + "macOS/" + str(px) + ".png", px, render("macOS", px))
  }
  manifest.icns.push((
    file: icns,
    members: sizes.macOS.members.map(m => (os: m.os, png: members + "macOS/" + str(m.px) + ".png")),
  ))

  let ico = expand(sizes.windows.file, flavor)
  for m in sizes.windows.members {
    png(members + "windows/" + str(m.px) + ".png", m.px, render("windows", m.px))
  }
  manifest.ico.push((file: ico, members: sizes.windows.members.map(m => members + "windows/" + str(m.px) + ".png")))

  let desktop = (
    sizes.linux.map(e => expand(e.file, flavor)) + (icns, ico)
  ).map(p => p.split("/").slice(0, -1).join("/")).dedup()
  manifest.owned += desktop.map(dir => dir + "/" + sizes.short.at(flavor) + "[-.]*")

  // --- iOS -------------------------------------------------------------------

  let ios = expand(sizes.iOS.dir, flavor)
  let images = ()
  for px in sizes.iOS.slots.map(s => s.px).dedup() {
    png(ios + "/icon-" + str(px) + ".png", px, render("iOS", px))
    manifest.place.push(ios + "/icon-" + str(px) + ".png")
  }
  for s in sizes.iOS.slots {
    images.push((idiom: s.idiom, scale: s.scale, size: s.size, filename: "icon-" + str(s.px) + ".png"))
  }
  png(ios + "/icon-tinted-1024.png", 1024, render("iOS", 1024, style: (fills.mono, app.background-inner, none)))
  manifest.place.push(ios + "/icon-tinted-1024.png")
  images.push((
    idiom: "universal",
    platform: "ios",
    size: "1024x1024",
    scale: "1x",
    filename: "icon-tinted-1024.png",
    appearances: ((appearance: "luminosity", value: "tinted"),),
  ))
  asset(ios + "/Contents.json", json.encode((images: images, info: (version: 1, author: "waypoints"))) + "\n")
  manifest.place.push(ios + "/Contents.json")
  // `**/*`, not `**`: walking the filesystem, `**` alone yields directories.
  manifest.owned.push(ios + "/**/*")

  // --- Android ---------------------------------------------------------------

  let res = expand(sizes.android.dir, flavor)
  for d in sizes.android.densities {
    let mipmap = "mipmap-" + d.density
    for (layer, theme) in (("foreground", fills.dark), ("monochrome", fills.mono)) {
      let staged = members + "android/" + mipmap + "/" + layer + ".png"
      png(staged, d.px, render("android", d.px, style: (theme, none, none)))
      manifest.webp.push((from: staged, to: res + "/" + mipmap + "/ic_launcher_" + layer + ".webp"))
    }
    manifest.owned.push(res + "/" + mipmap + "/ic_launcher_*")
  }

  let adaptive = (
    "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n"
      + "<adaptive-icon xmlns:android=\"http://schemas.android.com/apk/res/android\">\n"
      + "  <background android:drawable=\"@color/ic_launcher_background\"/>\n"
      + "  <foreground android:drawable=\"@mipmap/ic_launcher_foreground\"/>\n"
      + "  <monochrome android:drawable=\"@mipmap/ic_launcher_monochrome\"/>\n"
      + "</adaptive-icon>\n"
  )
  for name in ("ic_launcher.xml", "ic_launcher_round.xml") {
    asset(res + "/mipmap-anydpi-v26/" + name, adaptive)
    manifest.place.push(res + "/mipmap-anydpi-v26/" + name)
  }
  manifest.owned.push(res + "/mipmap-anydpi-v26/ic_launcher*")

  let background = (
    "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n"
      + "<resources>\n"
      + "  <color name=\"ic_launcher_background\">" + argb(app.background-inner) + "</color>\n"
      + "</resources>\n"
  )
  asset(res + "/values/ic_launcher_background.xml", background)
  manifest.place.push(res + "/values/ic_launcher_background.xml")
  manifest.owned.push(res + "/values/ic_launcher_background.xml")
}

// Rendering every flavor owns the desktop directory outright, so a file that
// matches no flavor is pruned too.
#if flavors.sorted() == all-flavors.sorted() {
  let dirs = (
    (sizes.linux.map(e => e.file) + (sizes.macOS.file, sizes.windows.file))
      .map(p => p.split("/").slice(0, -1).join("/"))
      .dedup()
  )
  manifest.owned = dirs.map(d => d + "/**/*") + manifest.owned.filter(p => not dirs.any(d => p.starts-with(d + "/")))
}

#asset("manifest.json", json.encode(manifest) + "\n")
