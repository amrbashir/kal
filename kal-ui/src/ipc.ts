export enum IpcAction {
  Query = "Query",
  ClearResults = "ClearResults",
  Execute = "Execute",
  ShowItemInDir = "ShowItemInDir",
  Reload = "Reload",
  HideMainWindow = "HideMainWindow",
}

export enum IpcEvent {
  FocusInput = "FocusInput",
  UpdateConfig = "UpdateConfig",
  UpdateSystemAccentColor = "UpdateSystemAccentColor",
}
