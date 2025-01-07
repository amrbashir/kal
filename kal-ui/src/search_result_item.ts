import type { Icon } from "./utils.ts";

export interface SearchResultItem {
  primary_text: string;
  secondary_text: string;
  icon: Icon;
  needs_confirmation: boolean;
  id: string;
}
