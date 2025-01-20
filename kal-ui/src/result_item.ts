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

export interface Action {
  id: string;
  icon?: Icon;
  accelerator?: string;
  description?: string;
}

export interface ResultItem {
  id: string;
  icon: Icon;
  primary_text: string;
  secondary_text: string;
  tooltip?: string;
  actions: Action[];
}
