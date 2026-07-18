/**
 * Classify whatever a user hands the setup flow — a paste, a scan, or an
 * incoming deep link — into the action it drives. One sniffer feeds every
 * entry point so the QR, the paste box, and the camera all accept the same
 * shapes.
 *
 * Recognized today: raw pairing-payload JSON (the shape
 * `outpost::vessel::PairingPayload::decode` validates for real — this
 * classifier only routes, it never trusts).
 */
export type SetupCode =
  /** A pairing payload (JSON string) for `ipc.sync.pairingJoin`. */
  | { kind: "pairing"; code: string }
  /** Unrecognized — surface a "not a setup code" error to the user. */
  | { kind: "unknown" };

export function classifySetupCode(input: string): SetupCode {
  const trimmed = input.trim();
  if (!trimmed) return { kind: "unknown" };

  if (trimmed.startsWith("{")) {
    return { kind: "pairing", code: trimmed };
  }

  return { kind: "unknown" };
}
