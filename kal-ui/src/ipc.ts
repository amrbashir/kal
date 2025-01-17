export enum IpcAction {
  Query = "Query",
  ClearResults = "ClearResults",
  Execute = "Execute",
  ShowItemInDir = "ShowItemInDir",
  RefreshIndex = "RefreshIndex",
  HideMainWindow = "HideMainWindow",
}

export enum IpcEvent {
  FocusInput = "FocusInput",
  UpdateConfig = "UpdateConfig",
  UpdateSystemAccentColor = "UpdateSystemAccentColor",
}
