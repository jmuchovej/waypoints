import type { IdentityCreated, StationStatus, SyncDevice, SyncStatus } from "@waypoints/cartograph";
import { invoke } from "./invoke";

export async function status(): Promise<SyncStatus> {
  return invoke<SyncStatus>("sync_status");
}

export async function identityCreate(): Promise<IdentityCreated> {
  return invoke<IdentityCreated>("sync_identity_create");
}

export async function identityImport(phrase: string): Promise<SyncStatus> {
  return invoke<SyncStatus>("sync_identity_import", { phrase });
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

export async function pairingPayload(): Promise<string> {
  return invoke<string>("sync_pairing_payload");
}

export async function pairingJoin(code: string): Promise<SyncStatus> {
  return invoke<SyncStatus>("sync_pairing_join", { code });
}
