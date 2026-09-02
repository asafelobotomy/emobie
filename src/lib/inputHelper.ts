export type InputHelperStatus = {
  daemon: boolean;
  canInject: boolean;
  canListen: boolean;
  detail: string;
  /** True when the app is running inside Flatpak. */
  flatpak?: boolean;
  /**
   * True when group `emobie-input` and system udev rules are installed.
   * Can be false even when `canListen` is true (temporary ACL / orphaned GID).
   */
  accessConfigured?: boolean;
};

export type InputMatch = {
  trigger: string;
  expansion: string;
  mode: "immediate" | "space";
};
