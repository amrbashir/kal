import { Action } from "./result_item";

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
  UpdateCustomCSS = "UpdateCustomCSS",
}

export async function runAction(action: Action, itemId: string) {
  const payload = `${action.id}#${itemId}`;
  await window.KAL.ipc.invoke(IpcCommand.RunAction, payload);
}
