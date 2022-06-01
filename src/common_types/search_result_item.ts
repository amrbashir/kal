import { Icon } from "./icon";

export interface SearchResultItem {
  primary_text: string;
  secondary_text: string;
  execution_args: string[];
  plugin_name: string;
  icon: Icon;
}
