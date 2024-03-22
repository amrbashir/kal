import { Icon } from "./icon";

export interface SearchResultItem {
  primary_text: string;
  secondary_text: string;
  icon: Icon;
  needs_confirmation: boolean;
  identifier: string;
}
