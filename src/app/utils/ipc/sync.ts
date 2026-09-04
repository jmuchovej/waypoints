import type {
  IdentityCreated,
  PairingCode,
  SetupCode,
  StationStatus,
  SyncDevice,
  SyncState,
  SyncStatus,
} from "@waypoints/cartograph";

import { invoke } from "./invoke";

export async function status(): Promise<SyncStatus> {
  return invoke<SyncStatus>("sync_status");
}

/**
 * Runtime sync observability: last sync, pending publications, deferred
 * operations, log growth, and live-session connectivity.
 */
export async function state(): Promise<SyncState> {
  return invoke<SyncState>("sync_state");
}

export async function identityCreate(): Promise<IdentityCreated> {
  return invoke<IdentityCreated>("sync_identity_create");
}

export async function identityMnemonic(): Promise<string> {
  return invoke<string>("sync_identity_mnemonic");
}

export async function territoryCreate(): Promise<SyncStatus> {
  return invoke<SyncStatus>("sync_territory_create");
}

export async function stationSet(addrJson: string): Promise<StationStatus> {
  return invoke<StationStatus>("sync_station_set", { addrJson });
}

export async function stationStatus(): Promise<StationStatus> {
  return invoke<StationStatus>("sync_station_status");
}

export async function syncNow(): Promise<SyncStatus> {
  return invoke<SyncStatus>("sync_now");
}

export async function devices(): Promise<SyncDevice[]> {
  return invoke<SyncDevice[]>("sync_devices");
}

/** The Add-device pairing code: shareable URL + its QR SVG. */
export async function pairingCode(): Promise<PairingCode> {
  return invoke<PairingCode>("sync_pairing_code");
}

/** QR SVG of the recovery phrase, for the Emergency Kit. */
export async function recoveryQr(): Promise<string> {
  return invoke<string>("sync_recovery_qr");
}

/** Classify a paste/scan/deep-link input — routing only, no side effects. */
export async function classifySetupCode(input: string): Promise<SetupCode> {
  return invoke<SetupCode>("sync_classify_setup_code", { input });
}

/** Classify-and-dispatch a setup code (pairing join or phrase recovery). */
export async function submitSetupCode(input: string): Promise<SyncStatus> {
  return invoke<SyncStatus>("sync_submit_setup_code", { input });
}
