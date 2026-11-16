import type { DeletedMacro, Macro } from "../types/preferences";
import { findHotkeyConflict, findTriggerConflict } from "./macroHelpers.ts";

/** Deleted macros kept for restoring. Keep in sync with `DELETED_MAX` in src-tauri/src/cli/macros.rs. */
export const DELETED_MACROS_MAX = 50;

export type MacroBin = { macros: Macro[]; deletedMacros: DeletedMacro[] };

/** Deletes `id`: it leaves the macros and goes first in the bin (the oldest drop out). */
export function trashMacro(
  macros: Macro[],
  deleted: DeletedMacro[],
  id: string,
  now: number,
): MacroBin {
  const gone = macros.find((macro) => macro.id === id);
  if (!gone) return { macros, deletedMacros: deleted };
  return {
    macros: macros.filter((macro) => macro.id !== id),
    deletedMacros: [
      { ...gone, deletedAt: now },
      ...deleted.filter((macro) => macro.id !== id),
    ].slice(0, DELETED_MACROS_MAX),
  };
}

/**
 * Brings `id` back. Refused when another macro took its trigger meanwhile; a hotkey that has
 * been taken (or became the summon shortcut) is left off instead.
 */
export function restoreDeletedMacro(
  macros: Macro[],
  deleted: DeletedMacro[],
  id: string,
  summonHotkey: string,
): MacroBin | { error: string } {
  const back = deleted.find((macro) => macro.id === id);
  if (!back) return { error: "That macro is no longer in the deleted list." };
  if (findTriggerConflict(macros, back.trigger, back.id)) {
    return { error: `Another macro already uses ${back.trigger}.` };
  }
  const { deletedAt: _at, ...macro } = back;
  const restored: Macro = findHotkeyConflict(macros, summonHotkey, macro.hotkey, macro.id)
    ? { ...macro, hotkey: null }
    : macro;
  return {
    macros: [...macros, restored],
    deletedMacros: deleted.filter((item) => item.id !== id),
  };
}

/** Removes one deleted macro for good, or all of them (`id` null). */
export function purgeDeleted(
  deleted: DeletedMacro[],
  id: string | null,
): DeletedMacro[] {
  return id === null ? [] : deleted.filter((macro) => macro.id !== id);
}
