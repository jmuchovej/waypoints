/**
 * Classify whatever a user hands the setup flow — a paste, a scan, or an
 * incoming deep link — into the action it drives. One sniffer feeds every
 * entry point so the QR, the paste box, and the camera all accept the same
 * shapes.
 *
 * Recognized: raw pairing-payload JSON (the shape
 * `outpost::vessel::PairingPayload::decode` validates for real — this
 * classifier only routes, it never trusts), a bare BIP-39 recovery phrase
 * (word-count shape only; the backend's mnemonic parse is the validation),
 * and the pairing URL envelope — `https://waypoints.so/pair#…` or
 * `waypoints://pair#…` with the payload JSON base64url-encoded in the
 * fragment. The fragment placement is deliberate: fragments never ride an
 * HTTP request, so a link opened in a browser leaks nothing to the server.
 */
export type SetupCode =
  /** A pairing payload (JSON string) for `ipc.sync.pairingJoin`. */
  | { kind: "pairing"; code: string }
  /** A recovery phrase for `ipc.sync.identityImport`. */
  | { kind: "phrase"; phrase: string }
  /** Unrecognized — surface a "not a setup code" error to the user. */
  | { kind: "unknown" };

/** Valid BIP-39 mnemonic lengths. */
const PHRASE_WORD_COUNTS = new Set([12, 15, 18, 21, 24]);

/** The two URL heads a pairing link may wear: the universal link (opens
 * the app when installed, a landing page when not) and the custom scheme
 * (registered by the desktop/mobile app). */
const PAIR_URL_HEADS = ["https://waypoints.so/pair", "waypoints://pair"];

export function classifySetupCode(input: string): SetupCode {
  const trimmed = input.trim();
  if (!trimmed) return { kind: "unknown" };

  if (trimmed.startsWith("{")) {
    return { kind: "pairing", code: trimmed };
  }

  const fragment = pairUrlFragment(trimmed);
  if (fragment != null) {
    const code = decodeBase64Url(fragment);
    return code == null ? { kind: "unknown" } : { kind: "pairing", code };
  }

  const words = trimmed.toLowerCase().split(/\s+/);
  if (PHRASE_WORD_COUNTS.has(words.length) && words.every((word) => /^[a-z]+$/.test(word))) {
    return { kind: "phrase", phrase: words.join(" ") };
  }

  return { kind: "unknown" };
}

/** Wrap a pairing payload (JSON string) into the shareable universal-link
 * form — what the Add-device QR encodes. */
export function encodePairUrl(payloadJson: string): string {
  return `${PAIR_URL_HEADS[0]}#${encodeBase64Url(payloadJson)}`;
}

function pairUrlFragment(input: string): string | null {
  for (const head of PAIR_URL_HEADS) {
    if (!input.startsWith(head)) continue;
    const hash = input.indexOf("#");
    if (hash === -1) return null;
    return input.slice(hash + 1);
  }
  return null;
}

function encodeBase64Url(text: string): string {
  const bytes = new TextEncoder().encode(text);
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}

function decodeBase64Url(encoded: string): string | null {
  try {
    const binary = atob(encoded.replaceAll("-", "+").replaceAll("_", "/"));
    const bytes = Uint8Array.from(binary, (c) => c.charCodeAt(0));
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    return null;
  }
}
