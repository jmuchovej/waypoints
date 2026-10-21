#import "screen.typ": screen

// A surface is one store's output slot: where its files land and how big they
// are. Content is declared once; this is the axis it is rendered across.
//
// `dir` is a function of the locale because the two stores disagree about where
// the locale sits in the path — supply nests images under the locale, deliver
// keeps screenshots in a tree of their own.
#let phone-surfaces = (
  (
    store: "Google Play — phone",
    dir: loc => "metadata/android/" + loc + "/images/phoneScreenshots",
    width: 1080pt,
    height: 1920pt,
    // The longest screen's body measures 1163pt at this card's interior width,
    // so the card needs 66% of 1920 to hold it without clipping.
    band: 28%,
    card: 66%,
    spec: "1080×1920 · 9:16, no alpha",
  ),
  (
    store: "App Store — iPhone 6.9\"",
    // deliver reads every image under the locale and picks the device class
    // from the resolution, so 1320×2868 lands as 6.9-inch on its own.
    dir: loc => "screenshots/" + loc,
    width: 1320pt,
    height: 2868pt,
    // Taller than 9:16 while type only grows with width, so the same content
    // needs a smaller share of the height here than it does on Play.
    band: 32%,
    card: 55%,
    spec: "1320×2868 · 6.9-inch iPhone, no alpha",
  ),
)

// Every screen onto every surface, so a new store is a row above rather than a
// second set of screens to keep in step. Both stores order screenshots by
// filename, hence the numeric prefix.
#let phone-assets(screens, locale, surfaces: phone-surfaces) = {
  let out = ()
  for surface in surfaces {
    for (i, s) in screens.enumerate(start: 1) {
      let nn = if i < 10 { "0" + str(i) } else { str(i) }
      out.push((
        group: surface.store,
        path: (surface.dir)(locale) + "/" + nn + "-" + s.slug + ".png",
        spec: surface.spec,
        body: screen(surface, s.headline, s.sub, s.body),
      ))
    }
  }
  out
}
