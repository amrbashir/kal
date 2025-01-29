export enum IconType {
  /** {@linkcode Icon.data} is the path to icon. */
  Path = "Path",
  /** {@linkcode Icon.data} is the path to extract icon from. */
  ExtractFromPath = "ExtractFromPath",
  /**
   * {@linkcode Icon.data} is a combination of two icons where the
   * the second icon is overlayed on top with half size.
   */
  Overlay = "Overlay",
  /** {@linkcode Icon.data} is an SVG string. */
  Svg = "Svg",
  /** {@linkcode Icon.data} is a {@linkcode BuiltinIcon} variant. */
  Builtin = "Builtin",
  /** {@linkcode Icon.data} is a url to an icon. */
  Url = "Url",
}

export interface Icon {
  data: string;
  type: IconType;
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
