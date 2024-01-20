import { Icon } from "./icon";

export interface SearchResultItem {
  /** The main text to be displayed for this item */
  primary_text: string;
  /** The secondary text to be displayed for this item */
  secondary_text: string;
  execution_args: any;
  plugin_name: string;
  /** The icon to display next to this item */
  icon: Icon;
  /** Whether execution of this item, requires confirmation or not */
  needs_confirmation: boolean;
}
