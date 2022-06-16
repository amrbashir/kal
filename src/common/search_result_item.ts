import { Icon } from "./icon";

export interface SearchResultItem {
  /** The main text to be displayed for this item */
  primary_text: string;
  /** The secondary text to be displayed for this item */
  secondary_text: string;
  execution_args: string[];
  plugin_name: string;
  /** The icon to display next to this item */
  icon: Icon;
}
