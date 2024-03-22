import { Icon, IconKind } from "../common";

export function convertFileSrc(protocol: string, filePath: string): string {
  const path = encodeURIComponent(filePath);
  return navigator.userAgent.includes("Windows")
    ? `http://${protocol}.localhost/${path}`
    : `${protocol}://${path}`;
}

export function getIconHtml(icon: Icon): string {
  switch (icon.kind) {
    case IconKind.Svg:
      return icon.data;
    case IconKind.Default:
    case IconKind.Path:
    default:
      return `<img src="${convertFileSrc("kalasset", icon.data)}" />`;
  }
}

export function isVScrollable<T extends Element>(el: T | null): boolean {
  console.log(el?.scrollHeight, el?.clientHeight);
  return el ? el.scrollHeight > el.clientHeight : false;
}
