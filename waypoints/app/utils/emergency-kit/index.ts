import { qrcode } from "etiket/qr";
import kitSource from "./emergency-kit.typ?raw";
import { loadTypst } from "../typst";

/**
 * The Waypoints Emergency Kit — see `emergency-kit.typ` for the document
 * itself (designable/previewable standalone). This module compiles it with
 * the same typst.ts WASM compiler the editor already ships: real values go
 * in through `sys.inputs`, and the real QR (etiket SVG) is shadow-mapped
 * over the design-time `demo-qr.svg`. Fully offline.
 */
export interface EmergencyKitInput {
  mnemonic: string;
  memberId: string;
}

// The .typ is mounted in the compiler's virtual filesystem at the same
// path it has inside the app source (mainContent would land in a random
// /tmp file), so its relative references — `demo-qr.svg` beside it and
// `../../../public/logos/…` — resolve exactly like they do for the
// design-time `typst compile --root waypoints/` workflow.
const MAIN_PATH = "/app/utils/emergency-kit/main.typ";
const QR_PATH = "/app/utils/emergency-kit/demo-qr.svg";
const LOGO_PATH = "/public/logos/wordmark-dark-md.svg";

export async function buildEmergencyKit(input: EmergencyKitInput): Promise<Uint8Array> {
  const phrase = input.mnemonic.trim();
  const created = new Date().toLocaleDateString(undefined, {
    year: "numeric",
    month: "long",
    day: "numeric",
  });

  // QR of the phrase itself — scan it to avoid typing 24 words. Plain
  // squares on purpose: maximum scanner compatibility for a document that
  // may be a crumpled printout in ten years.
  const qrSvg = qrcode(phrase, { ecLevel: "M", margin: 0 });

  // The wordmark ships as an app asset; feed it to the compiler from here.
  const logo = await fetch("/logos/wordmark-dark-md.svg").then((r) => {
    if (!r.ok) throw new Error(`failed to load wordmark: ${r.status}`);
    return r.arrayBuffer();
  });

  const $typst = await loadTypst();
  await $typst.addSource(MAIN_PATH, kitSource);
  await $typst.mapShadow(QR_PATH, new TextEncoder().encode(qrSvg));
  await $typst.mapShadow(LOGO_PATH, new Uint8Array(logo));
  try {
    const pdf = await $typst.pdf({
      mainFilePath: MAIN_PATH,
      inputs: {
        kit: JSON.stringify({
          memberId: input.memberId,
          words: phrase.split(/\s+/),
          created,
        }),
      },
    });
    if (!pdf) throw new Error("typst compilation produced no PDF");
    return pdf;
  } finally {
    await $typst.unmapShadow(QR_PATH);
    await $typst.unmapShadow(LOGO_PATH);
  }
}

/**
 * Save the kit: native save dialog under Tauri, a plain download in the
 * browser. Returns false when the user cancelled the dialog.
 */
export async function saveEmergencyKit(
  bytes: Uint8Array,
  filename = "waypoints-emergency-kit.pdf",
): Promise<boolean> {
  if ("isTauri" in window) {
    const { save } = await import("@tauri-apps/plugin-dialog");
    const path = await save({
      defaultPath: filename,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (!path) return false;
    const { writeFile } = await import("@tauri-apps/plugin-fs");
    await writeFile(path, bytes);
    return true;
  }

  const blob = new Blob([bytes as BlobPart], { type: "application/pdf" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
  return true;
}
