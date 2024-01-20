export interface Icon {
  /** Could be a path or an svg based on the {@link kind} field */
  data: string;
  kind: IconKind;
}

export enum IconKind {
  Path = "Path",
  Svg = "Svg",
  Default = "Default",
}
