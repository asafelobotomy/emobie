/**
 * Under uQ, Super+. opens uQ's own emoji search (emobie-provider), so emobie's
 * summon hotkey stays off to avoid a double bind. `uqDesktop` is null until the
 * check (org.uq.Desktop1 on the session bus) has answered, so the hotkey is not
 * registered and then dropped again on a uQ desktop.
 */
export function summonHotkeyEnabled(options: {
  ready: boolean;
  allowMultipleInstances: boolean;
  uqDesktop: boolean | null;
}): boolean {
  return (
    options.ready && !options.allowMultipleInstances && options.uqDesktop === false
  );
}
