import { invoke } from "@tauri-apps/api/core";
import { load, type Store } from "@tauri-apps/plugin-store";
import {
  type Preferences,
} from "../types/preferences";
import { mergePreferencePartials, userDataFingerprint } from "./mergePreferences";
import { normalizePreferences } from "./normalizePreferences";

const STORE_PATH = "emobie-preferences.json";

type PreferenceSnapshot = {
  source: string;
  preferences: Partial<Preferences>;
};

let storePromise: Promise<Store> | null = null;

function getStore(): Promise<Store> {
  if (!storePromise) {
    storePromise = load(STORE_PATH, { autoSave: true }).catch((error) => {
      storePromise = null;
      throw error;
    });
  }
  return storePromise;
}

async function loadSnapshots(): Promise<{
  ok: boolean;
  snapshots: PreferenceSnapshot[];
}> {
  try {
    const snapshots = await invoke<PreferenceSnapshot[]>(
      "load_preference_snapshots",
    );
    return { ok: true, snapshots };
  } catch (error) {
    console.warn("Could not load preference snapshots", error);
    return { ok: false, snapshots: [] };
  }
}

export async function readPreferences(): Promise<Preferences> {
  let primary: Partial<Preferences> | undefined;
  let primaryLoaded = false;
  try {
    const store = await getStore();
    primary = (await store.get<Partial<Preferences>>("preferences")) ?? {};
    primaryLoaded = true;
  } catch (error) {
    console.warn("Could not read app preference store", error);
  }

  const { ok: snapshotsOk, snapshots } = await loadSnapshots();
  const extras = snapshots.map((snap) => snap.preferences);
  const merged = normalizePreferences(
    mergePreferencePartials(primary, extras),
  );

  const primaryFp = userDataFingerprint(primary ?? {});
  const mergedFp = userDataFingerprint(merged);

  // Persist recovered macros/favorites/recents back into the active store.
  if (mergedFp !== primaryFp) {
    await writePreferences(merged);
  } else if (primaryLoaded && snapshotsOk) {
    // Refresh durable mirror only when we successfully read every source.
    await writeDurableOnly(merged);
  }

  return merged;
}

async function writeDurableOnly(prefs: Preferences): Promise<void> {
  try {
    await invoke("save_durable_preferences", { preferences: prefs });
  } catch (error) {
    console.warn("Could not write durable preferences", error);
  }
}

export async function writePreferences(
  prefs: Preferences,
  writeRev = 0,
): Promise<boolean> {
  let storeOk = false;
  try {
    const store = await getStore();
    await store.set("preferences", prefs);
    await store.save();
    storeOk = true;
  } catch (error) {
    console.error("Failed to save preferences", error);
  }

  try {
    await invoke("save_durable_preferences", {
      preferences: prefs,
      writeRev,
    });
  } catch (error) {
    console.error("Failed to save durable preferences", error);
    // Active store write still counts as success when durable mirror fails
    // (e.g. Flatpak without xdg-data/emobie write access).
    return storeOk;
  }

  return storeOk;
}
