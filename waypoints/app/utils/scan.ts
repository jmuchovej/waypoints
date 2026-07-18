/**
 * The scan seam: one "get me a decoded QR string" surface with two
 * implementations. Mobile uses the platform's native scanner
 * (tauri-plugin-barcode-scanner — camera permission and decoding handled
 * natively); desktop and browser dev fall back to the webview
 * getUserMedia scanner (SyncScanCode.vue). Both paths emit the same raw
 * text into the Rust setup-code engine, so the platforms cannot drift.
 */

/** Whether this platform scans natively (behind the webview). */
export async function isNativeScanPlatform(): Promise<boolean> {
  if (!("isTauri" in window)) return false;
  try {
    const { platform } = await import("@tauri-apps/plugin-os");
    const os = platform();
    return os === "android" || os === "ios";
  } catch {
    return false;
  }
}

/**
 * Run one native scan and resolve with the decoded text, or null when the
 * user cancelled / permission was refused. The camera renders BEHIND the
 * webview (`windowed: true`) — callers must present a see-through overlay
 * while this is pending.
 */
export async function nativeScan(): Promise<string | null> {
  const scanner = await import("@tauri-apps/plugin-barcode-scanner");

  let permission = await scanner.checkPermissions();
  if (permission === "prompt") {
    permission = await scanner.requestPermissions();
  }
  if (permission !== "granted") return null;

  const scanned = await scanner.scan({ windowed: true, formats: [scanner.Format.QRCode] });
  return scanned.content || null;
}

/** Abort an in-flight native scan (e.g. the overlay's cancel button). */
export async function cancelNativeScan(): Promise<void> {
  const { cancel } = await import("@tauri-apps/plugin-barcode-scanner");
  await cancel().catch(() => {});
}
