export type InputHelperStatus = {
  daemon: boolean;
  canInject: boolean;
  canListen: boolean;
  detail: string;
  /** True when the app is running inside Flatpak. */
  flatpak?: boolean;
};

export type InputMatch = {
  trigger: string;
  expansion: string;
  mode: "immediate" | "space";
};
