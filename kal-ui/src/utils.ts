import { Icon, IconType } from "./result_item";

export function makeIconHTML(icon: Icon): string {
  switch (icon.type) {
    case IconType.BuiltIn:
    case IconType.Svg:
      return icon.data;
    case IconType.Path:
      return `<img src="${window.KAL.ipc.makeProtocolFileSrc("kalicon", icon.data)}" />`;
    case IconType.Url:
      return `<img src="${icon.data}" />`;
    default:
      throw `Icon type \`${icon.type}\` not implemented`;
  }
}

export function isVScrollable<T extends Element>(el: T | null): boolean {
  return el ? el.scrollHeight > el.clientHeight : false;
}

export function isEventForHotkey(e: KeyboardEvent, accelerator: string): boolean {
  const modsAndkeys = accelerator.toLowerCase().split("+");

  if (modsAndkeys.length === 0) return false;

  const key = modsAndkeys[modsAndkeys.length - 1];

  if (e.key.toLowerCase() !== key) return false;

  const ctrl = e.ctrlKey == modsAndkeys.includes("ctrl");
  const shift = e.shiftKey == modsAndkeys.includes("shift");
  const alt = e.altKey == modsAndkeys.includes("alt");

  return ctrl && shift && alt;
}
