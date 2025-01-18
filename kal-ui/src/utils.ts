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
