export interface Icon {
  /** Could be a path or an svg based on the {@link type} field */
  data: string;
  type: IconType;
}

export enum IconType {
  Path = "Path",
  Svg = "Svg",
  Default = "Default",
}
