import { readFileSync } from "node:fs";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path, { dirname, resolve } from "node:path";
import { env } from "node:process";
import { fileURLToPath } from "node:url";

import { Icns, IcnsImage } from "@fiahfy/icns";
import chroma from "chroma-js";
import { defineCommand, runMain } from "citty";
import { parseTOML } from "confbox";
import pngToIco from "png-to-ico";
import sharp, { type Sharp } from "sharp";
import { type INode, parse as parseSvg, stringify as stringifySvg } from "svgson";
import tailwindColors from "tailwindcss/colors";

const HERE = dirname(fileURLToPath(import.meta.url));

interface TauriRoots {
  /** The Tauri app crate: holds `icons/` and, by default, `gen/`. */
  app: string;
  android: string;
  ios: string;
}

/**
 * Where Tauri places the app and its generated mobile projects.
 *
 * Read from the same variables `tauri {ios,android} build` reads, so relocating
 * a project moves the icons with it and nothing here encodes the repo layout.
 * Resolved on demand rather than at module load so `--help` still works when
 * the environment is not set up.
 */
function tauriRoots(): TauriRoots {
  const app = env.TAURI_APP_PATH;
  if (!app) {
    throw new Error(
      "TAURI_APP_PATH is unset — enter the devenv shell, or run this through `just icons::<flavor>`",
    );
  }
  return {
    app,
    android: env.TAURI_ANDROID_PROJECT_PATH || path.join(app, "gen/android"),
    ios: env.TAURI_IOS_PROJECT_PATH || path.join(app, "gen/apple"),
  };
}

type TailwindColors = typeof tailwindColors;
type TailwindColor = keyof TailwindColors;
type TailwindFamily = keyof Omit<
  TailwindColors,
  "transparent" | "inherit" | "current" | "black" | "white"
>;
type TailwindShade = keyof TailwindColors[TailwindFamily];
type Theme = "light" | "dark" | "mono";
type Flavor = "development" | "nightly" | "preview" | "stable";
type SourceKey = "xs" | "md";

interface LogoFill {
  SW: string;
  "SW→NW": string;
  NW: string;
  "NW→SE": string;
  SE: string;
  NE: string;
}

interface FlavorConfig {
  fills: Record<Theme, LogoFill>;
  app: {
    "background-inner": string;
    "background-outer": string;
  };
}

type Fill = LogoFill & {
  inner?: string;
  outer?: string;
};

type Fills = Record<Theme, Fill>;

const SIZES = parseTOML(readFileSync(resolve(HERE, `./sizes.toml`), "utf-8")) as Record<
  string,
  Record<string, string | number>
>;

const LINUX_PNGS = SIZES["linux"] as unknown as ReadonlyArray<{
  name: string;
  px: number;
}>;
const ICNS_ENTRIES = SIZES["macOS"] as unknown as ReadonlyArray<{
  os: string;
  px: number;
}>;
const ICO_SIZES = SIZES["windows"] as unknown as ReadonlyArray<{ px: number }>;

interface IosEntry {
  idiom: "iphone" | "ipad" | "ios-marketing";
  size: string; // point size, e.g. "20x20"
  scale: "1x" | "2x" | "3x";
  px: number;
}
const IOS_ENTRIES = SIZES["iOS"] as unknown as ReadonlyArray<IosEntry>;
const MENU_BAR_SCALES = SIZES["menu-bar"] as unknown as ReadonlyArray<{
  name: string;
  px: number;
}>;
const ANDROID_DENSITIES = SIZES["android"] as unknown as ReadonlyArray<{
  density: string;
  px: number;
}>;

const FLAVORS = ["development", "nightly", "preview", "stable"] as const;
const THEMES = ["light", "dark"] as const;
const PLATFORMS = ["windows", "linux", "macOS", "iOS", "android"] as const;
type Platform = (typeof PLATFORMS)[number];

/**
 * Pick which master SVG to rasterize from for a given target pixel size.
 *
 * The `xs` master is a simplified version of the logo without connector
 * lines between the four squircles — legible at small sizes where the
 * connectors would be visual noise. The `md` master has the full artwork.
 * 64px is the crossover point (matches macOS `icon_32x32@2x` → xs, any
 * 64×64 `@1x` slot → md, which is what the ICNS/iconset conventions use).
 */
function pickSource(px: number): SourceKey {
  return px <= 64 ? "xs" : "md";
}

async function loadLogoSVG(srcDir: string, px: number, fill: Fill): Promise<INode> {
  const suffix = pickSource(px);
  const path = resolve(srcDir, `icon/${suffix}.svg`);
  const file = await readFile(path, "utf-8");
  const tree = await parseSvg(file);
  walk(tree, (node) => {
    const id = node.attributes?.id;
    if (!id || !(id in fill)) return;
    node.attributes.fill = fill[id as keyof Fill] as string;
    node.attributes.stroke = fill[id as keyof Fill] as string;
  });
  return tree;
}

// Mask structural IDs — these are handled by paintInnerBG, paintOuterBG,
// and injectLogo, not by the generic fill painter in loadMaskSVG.
const MASK_STRUCTURAL_IDS = new Set(["background", "inner", "inner-shadow", "drop-shadow"]);

async function loadMaskSVG(srcDir: string, platform: Platform, fill: Fill): Promise<INode> {
  const path = resolve(srcDir, `masks/${platform}.svg`);
  const file = await readFile(path, "utf-8");
  const tree = await parseSvg(file);
  walk(tree, (node) => {
    const id = node.attributes?.id;
    if (!id || MASK_STRUCTURAL_IDS.has(id) || !(id in fill)) return;
    node.attributes.fill = fill[id as keyof Fill] as string;
  });
  return tree;
}

function parsePlatforms(raw: string): Set<Platform> {
  const items = raw
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
  if (items.length === 0 || items.includes("all")) {
    return new Set(PLATFORMS);
  }
  const result = new Set<Platform>();
  for (const item of items) {
    if (!(PLATFORMS as readonly string[]).includes(item)) {
      throw new Error(`Unknown platform "${item}" (expected: ${PLATFORMS.join("|")}|all)`);
    }
    result.add(item as Platform);
  }
  return result;
}

function isFlavor(value: string): value is Flavor {
  return (FLAVORS as readonly string[]).includes(value);
}

function isTheme(value: string): value is Theme {
  return (THEMES as readonly string[]).includes(value);
}

type TailwindTokenMatch = {
  family: TailwindColor;
  shade?: TailwindShade;
  opacity?: string;
};

/**
 * Resolve a `family.shade` token (e.g. `emerald.400`) to a `#rrggbb` string.
 *
 * Tailwind v4 ships palette values as `oklch(...)` strings, so we run every
 * lookup through chroma-js to normalize to hex. Sharp and SVG `fill=` both
 * want a color string they can parse unambiguously; hex is the safest bet.
 */
function resolveToken(token: string): string {
  // Pass-through for literal hex (used by the synthetic mono palette).
  if (token.startsWith("#")) return token;

  const match = token.match(/(?<family>\w+)\.?(?<shade>\d+)?\/?(?<opacity>\d+)?/);
  if (!match?.groups) {
    throw new Error(`Could not find any <family>, <shade>, or <opacity> in ${token}!`);
  }
  const { family, shade, opacity } = match.groups as TailwindTokenMatch;
  if (!family) {
    throw new Error(`Malformed color token: "${token}" (expected "family")`);
  }

  let color: string | undefined;
  if (typeof tailwindColors[family] === "string") {
    color = tailwindColors[family];
  } else if (tailwindColors[family] && shade) {
    color = tailwindColors[family][shade];
  }

  if (!color) {
    throw new Error(`Unknown tailwind color: ${family}.${shade}...`);
  }

  const alpha = Math.round((Number(opacity ?? 100) / 100) * 255)
    .toString(16)
    .padStart(2, "0");

  try {
    return chroma(color).hex() + alpha;
  } catch (err) {
    throw new Error(
      `chroma-js failed to parse tailwind color "${token}": ${(err as Error).message}`,
    );
  }
}

/**
 * Walk an svgson tree depth-first, invoking `visit` on every element node.
 */
function walk(node: INode, visit: (n: INode) => void): void {
  visit(node);
  for (const child of node.children ?? []) walk(child, visit);
}

async function loadFlavor(sourceDir: string, flavor: Flavor, theme: Theme): Promise<FlavorConfig> {
  const path = resolve(sourceDir, `flavors/${flavor}.toml`);
  const raw = await readFile(path, "utf-8");
  const config = parseTOML(raw) as FlavorConfig;
  const fills = config.fills[theme];
  if (!fills) {
    throw new Error(`Flavor "${flavor}" has no fills.${theme} table`);
  }
  const innerToken = config.app["background-inner"];
  if (!innerToken) {
    throw new Error(`Flavor "${flavor}" has no app.background-inner`);
  }
  const outerToken = config.app["background-outer"];
  if (!outerToken) {
    throw new Error(`Flavor "${flavor}" has no app.background-outer`);
  }

  return config;
}

function flavorFill(config: FlavorConfig, theme: Theme): Fill {
  const fill = Object.entries(config.fills[theme]).map(([k, v]) => [k, resolveToken(v)]);
  return {
    ...Object.fromEntries(fill),
    inner: resolveToken(config.app["background-inner"] ?? "transparent"),
    outer: resolveToken(config.app["background-outer"] ?? "transparent"),
  } as Fill;
}

function paintInnerBG(tree: INode, inner: string | undefined): INode {
  let painted = false;
  walk(tree, (node) => {
    if (node.attributes?.id !== "background") return;
    node.attributes.fill = inner ?? "transparent";
    painted = true;
  });

  if (!painted) throw new Error(`Could not find \`#background\`!`);
  return tree;
}

function paintOuterBG(tree: INode, outer: string | undefined): INode {
  if (!outer) return tree;

  const ids = new Set<string>();
  walk(tree, (node) => {
    if (node.attributes?.id !== "inner-shadow") return;
    walk(node, (child) => {
      const f = child.attributes?.filter;
      const m = f?.match(/url\(#([^)]+)\)/);
      if (m) ids.add(m[1]);
    });
  });
  if (ids.size === 0) return tree;

  walk(tree, (node) => {
    if (node.name !== "filter") return;
    if (!ids.has(node.attributes?.id ?? "")) return;

    const matrices = (node.children ?? []).filter(
      (c) => c.name === "feColorMatrix" && c.attributes?.type === "matrix",
    );
    const tint = matrices[matrices.length - 1];
    if (!tint) return;

    const [r, g, b] = chroma(resolveToken(outer)).rgb();
    const norm = (v: number) => (v / 255).toFixed(9);
    tint.attributes.values = `
      0 0 0 0 ${norm(r)}
      0 0 0 0 ${norm(g)}
      0 0 0 0 ${norm(b)}
      0 0 0 1 0
    `.replace(/\s+/g, " ");
  });

  return tree;
}

function injectLogo(tree: INode, logo: INode, width: number): INode {
  let injected = false;
  walk(tree, (node) => {
    if (node.attributes?.id !== "inner") return;
    const { id, transform } = node.attributes;
    if (!transform) throw new Error(`Could not find \`transform\` on ${id}!`);

    const match = transform.match(/translate\(\s*([\d.]+)\s*[,\s]\s*([\d.]+)\s*\)/);
    if (!match) throw new Error(`no translate() on ${id}`);

    const tx = parseFloat(match[1]);
    const size = width - 2 * tx;
    const nested: INode = {
      name: "svg",
      type: "element",
      value: "",
      attributes: {
        ...logo.attributes,
        x: "0",
        y: "0",
        width: String(size),
        height: String(size),
      },
      children: logo.children,
    };
    node.children = [nested];
    // Strip Sketch's dummy xlink:href that references an empty path.
    if (node.attributes["xlink:href"]) delete node.attributes["xlink:href"];
    injected = true;
  });
  if (!injected) throw new Error("`#inner` not found");

  return tree;
}

/** Convert `#RRGGBBAA` → `#AARRGGBB` for Android color resources. */
function rgbaToArgb(hex: string): string {
  const h = hex.replace("#", "");
  if (h.length <= 6) return hex;
  return `#${h.slice(6)}${h.slice(0, 6)}`;
}

/** Rasterize an SVG buffer at the given size, preserving alpha. */
async function renderSvg(
  srcDir: string,
  platform: Platform,
  px: number,
  fill: Fill,
): Promise<Sharp> {
  const logo = await loadLogoSVG(srcDir, px, fill);
  const mask = await loadMaskSVG(srcDir, platform, fill);

  const width = parseFloat(mask.attributes?.width.replace(/px$/, ""));
  if (!Number.isFinite(width) || width <= 0) {
    throw new Error(`mask has no usable width! (got ${mask.attributes.width})`);
  }

  let svg = paintInnerBG(mask, fill.inner);
  svg = paintOuterBG(svg, fill.outer);
  svg = injectLogo(svg, logo, width);

  const str = stringifySvg(svg);
  return sharp(Buffer.from(str)).resize(px, px);
}

async function toPng(s: Sharp): Promise<Buffer<ArrayBufferLike>> {
  return await s.png().toBuffer();
}

async function toWebp(s: Sharp): Promise<Buffer<ArrayBufferLike>> {
  return await s.webp({ lossless: true }).toBuffer();
}

const main = defineCommand({
  meta: {
    name: "gen-icons",
    description: "Generate Waypoints app icons for a given flavor",
  },
  args: {
    flavor: {
      type: "string",
      required: true,
      description: `Flavor to build (${FLAVORS.join("|")})`,
    },
    theme: {
      type: "string",
      default: "dark",
      description: `Fill theme (${THEMES.join("|")})`,
    },
    "source-dir": {
      type: "string",
      default: HERE,
      description: "Directory containing <flavor>.toml and images/",
    },
    "out-dir": {
      type: "string",
      default: "",
      description:
        "Directory holding one subdirectory per flavor, named in Tauri.toml and the flavor configs (default: $TAURI_APP_PATH/icons)",
    },
    platforms: {
      type: "string",
      default: "linux,windows,macOS,iOS,android",
      description: `Comma-separated platforms to build (${PLATFORMS.join("|")}|all)`,
    },
  },
  async run({ args }) {
    const flavor = args.flavor;
    const theme = args.theme;
    if (!isFlavor(flavor)) {
      throw new Error(`Invalid flavor "${flavor}" (expected one of: ${FLAVORS.join(", ")})`);
    }
    if (!isTheme(theme)) {
      throw new Error(`Invalid theme "${theme}" (expected one of: ${THEMES.join(", ")})`);
    }
    const platforms = parsePlatforms(args.platforms);

    const tauri = tauriRoots();
    const srcDir = resolve(args["source-dir"]);
    const dstDir = args["out-dir"] ? resolve(args["out-dir"]) : resolve(tauri.app, "icons");

    const config = await loadFlavor(srcDir, flavor, theme);
    const fills: Record<Theme, Fill> = {
      mono: flavorFill(config, "mono"),
      dark: flavorFill(config, "dark"),
      light: flavorFill(config, "light"),
    };

    await mkdir(dstDir, { recursive: true });
    const writers: Record<Platform, Function> = {
      macOS: writeMacOS,
      windows: writeWindows,
      linux: writeLinux,
      iOS: writeiOS,
      android: writeAndroid,
    };
    for await (const platform of platforms) {
      await writers[platform](flavor, srcDir, dstDir, fills, tauri);
    }
    await writeMenuIcons(flavor, srcDir, dstDir, fills);
  },
});

async function writeLinux(
  flavor: Flavor,
  srcDir: string,
  dstDir: string,
  fills: Fills,
): Promise<void> {
  await mkdir(dstDir, { recursive: true });

  for await (const entry of LINUX_PNGS) {
    const svg = await renderSvg(srcDir, "linux", entry.px, fills["dark"]);
    await writeFile(resolve(dstDir, flavor, entry.name), await toPng(svg));
  }
}

async function writeWindows(
  flavor: Flavor,
  srcDir: string,
  dstDir: string,
  fills: Fills,
): Promise<void> {
  await mkdir(dstDir, { recursive: true });

  const buffers = await Promise.all(
    ICO_SIZES.map(async (size) => {
      const svg = await renderSvg(srcDir, "windows", size.px, fills["dark"]);
      return await toPng(svg);
    }),
  );

  const icos = await pngToIco(buffers);
  await writeFile(resolve(dstDir, flavor, "icon.ico"), icos);
}

async function writeMacOS(
  flavor: Flavor,
  srcDir: string,
  dstDir: string,
  fills: Fills,
): Promise<void> {
  await mkdir(dstDir, { recursive: true });

  const icns = new Icns();
  for await (const entry of ICNS_ENTRIES) {
    const png = await renderSvg(srcDir, "macOS", entry.px, fills["dark"]);
    icns.append(IcnsImage.fromPNG(await toPng(png), entry.os as any));
  }

  await writeFile(resolve(dstDir, flavor, "icon.icns"), icns.data);
}

async function writeMenuIcons(
  flavor: Flavor,
  srcDir: string,
  dstDir: string,
  fills: Fills,
): Promise<void> {
  const fill = fills["mono"];
  // for await (const scale of MENU_BAR_SCALES) {
  //   const logo = await loadLogoSVG(srcDir, scale.px, fill);
  //   const png = await renderSvg(srcDir, "macOS", scale.px, fills["mono"]);
  //   await writeFile(resolve(dstDir, scale.name), png);
  // }
}

type iOSManifestEntry = {
  idiom: string;
  scale: string;
  size: string;
  filename: string;
  platform?: string;
  appearances?: Array<{ appearance: string; value: string }>;
};
async function writeiOS(
  flavor: Flavor,
  srcDir: string,
  dstDir: string,
  fills: Fills,
  tauri: TauriRoots,
): Promise<void> {
  const iosDir = resolve(tauri.ios, `Assets.xcassets/${flavor}.appiconset`);
  await mkdir(iosDir, { recursive: true });

  const manifest: Array<iOSManifestEntry> = [];
  const writtenByPx = new Map<number, string>();

  for await (const entry of IOS_ENTRIES) {
    let filename = writtenByPx.get(entry.px);
    if (!filename) {
      filename = `icon-${entry.px}.png`;
      const png = await toPng(await renderSvg(srcDir, "iOS", entry.px, fills["dark"]));
      await writeFile(resolve(iosDir, filename), png);
      writtenByPx.set(entry.px, filename);
    }
    manifest.push({
      idiom: entry.idiom,
      scale: entry.scale,
      size: entry.size,
      filename,
    });
  }

  const px = 1024;
  const filename = "icon-tinted-1024.png";
  const png = await toPng(await renderSvg(srcDir, "iOS", px, fills["mono"]));
  await writeFile(resolve(iosDir, filename), png);
  manifest.push({
    idiom: "universal",
    platform: "ios",
    size: `${px}x${px}`,
    scale: "1x",
    filename,
    appearances: [{ appearance: "luminosity", value: "tinted" }],
  });

  await writeFile(
    resolve(iosDir, "Contents.json"),
    JSON.stringify(
      {
        images: manifest,
        info: { version: 1, author: "waypoints" },
      },
      null,
      2,
    ) + "\n",
  );
}

async function writeAndroid(
  flavor: Flavor,
  srcDir: string,
  dstDir: string,
  fills: Fills,
  tauri: TauriRoots,
): Promise<void> {
  const resDir = resolve(tauri.android, `app/src/${flavor}/res`);

  // Foreground layer (logo on transparent bg) + monochrome silhouette per
  // mipmap density, as WebP. The monochrome layer is what Material You
  // tints at runtime when the user enables themed icons (Android 13+).
  for (const { density, px } of ANDROID_DENSITIES) {
    const dir = resolve(resDir, `mipmap-${density}`);
    await mkdir(dir, { recursive: true });

    var fill = { ...fills["dark"], inner: undefined, outer: undefined };
    const fg = await toWebp(await renderSvg(srcDir, "android", px, fill));
    await writeFile(resolve(dir, "ic_launcher_foreground.webp"), fg);

    var fill = { ...fills["mono"], inner: undefined, outer: undefined };
    const mono = await toWebp(await renderSvg(srcDir, "android", px, fill));
    await writeFile(resolve(dir, "ic_launcher_monochrome.webp"), mono);
  }

  // mipmap-anydpi-v26/ic_launcher{,_round}.xml — the adaptive-icon glue.
  const anydpiDir = resolve(resDir, "mipmap-anydpi-v26");
  await mkdir(anydpiDir, { recursive: true });
  const adaptiveXML = `<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@color/ic_launcher_background"/>
    <foreground android:drawable="@mipmap/ic_launcher_foreground"/>
    <monochrome android:drawable="@mipmap/ic_launcher_monochrome"/>
</adaptive-icon>
`;
  await writeFile(resolve(anydpiDir, "ic_launcher.xml"), adaptiveXML);
  await writeFile(resolve(anydpiDir, "ic_launcher_round.xml"), adaptiveXML);

  const valuesDir = resolve(resDir, "values");
  await mkdir(valuesDir, { recursive: true });
  const bgXML = `<?xml version="1.0" encoding="utf-8"?>
<resources>
    <color name="ic_launcher_background">${rgbaToArgb(fills["dark"].inner!)}</color>
</resources>
`;
  await writeFile(resolve(valuesDir, "ic_launcher_background.xml"), bgXML);
}

runMain(main);
