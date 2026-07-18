/**
 * Classify whatever a user hands the setup flow — a paste, a scan, or an
 * incoming deep link — into the action it drives. One sniffer feeds every
 * entry point so the QR, the paste box, and the camera all accept the same
 * shapes.
 *
 * Recognized: raw pairing-payload JSON (the shape
 * `outpost::vessel::PairingPayload::decode` validates for real — this
 * classifier only routes, it never trusts) and a bare BIP-39 recovery
 * phrase (word-count shape only; the backend's mnemonic parse is the
 * validation).
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

export function classifySetupCode(input: string): SetupCode {
  const trimmed = input.trim();
  if (!trimmed) return { kind: "unknown" };

  if (trimmed.startsWith("{")) {
    return { kind: "pairing", code: trimmed };
  }

  const words = trimmed.toLowerCase().split(/\s+/);
  if (PHRASE_WORD_COUNTS.has(words.length) && words.every((word) => /^[a-z]+$/.test(word))) {
    return { kind: "phrase", phrase: words.join(" ") };
  }

  return { kind: "unknown" };
}
