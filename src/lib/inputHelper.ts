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
  /** In-flight expand jobs holding listen suppress (debug). */
  suppressJobs?: number;
  /** Clipboard restore after paste (default false). */
  restoreClipboard?: boolean;
  /** Last expand insert backend: keys | ei | wl-copy | arboard. */
  lastInjectBackend?: string;
};

export type InputMatch = {
  trigger: string;
  expansion: string;
  mode: "immediate" | "space";
};
