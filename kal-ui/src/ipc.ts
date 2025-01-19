export enum IpcCommand {
  Query = "Query",
  ClearResults = "ClearResults",
  RunAction = "RunAction",
  Reload = "Reload",
  HideMainWindow = "HideMainWindow",
}

export enum IpcEvent {
  FocusInput = "FocusInput",
  UpdateConfig = "UpdateConfig",
  UpdateSystemAccentColor = "UpdateSystemAccentColor",
}
