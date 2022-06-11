export interface Icon {
  data: string;
  type: IconType;
}

export enum IconType {
  Path = "Path",
  Svg = "Svg",
}
