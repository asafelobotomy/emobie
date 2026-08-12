export type InputHelperStatus = {
  daemon: boolean;
  canInject: boolean;
  canListen: boolean;
  detail: string;
};

export type InputMatch = {
  trigger: string;
  expansion: string;
};
