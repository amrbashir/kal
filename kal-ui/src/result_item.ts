export interface Icon {
  data: string;
  type: IconType;
}

export enum IconType {
  Path = "Path",
  Svg = "Svg",
  BuiltIn = "BuiltIn",
  Url = "Url",
}

export interface ResultItem {
  primary_text: string;
  secondary_text: string;
  icon: Icon;
  needs_confirmation: boolean;
  id: string;
}
