import { Icon, IconType } from "../common";

export function convertFileSrc(protocol: string, filePath: string): string {
  const path = encodeURIComponent(filePath);
  return navigator.userAgent.includes("Windows")
    ? `http://${protocol}.localhost/${path}`
    : `${protocol}://${path}`;
}

export function getIconHtml(icon: Icon): string {
  switch (icon.type) {
    case IconType.Svg:
      return icon.data;
    case IconType.Url:
      return `<img src="${icon.data}" />`;
    case IconType.BuiltinIcon:
      return `<img src="${convertFileSrc("kalasset", icon.data)}?type=builtin" />`;
    case IconType.Path:
    default:
      return `<img src="${convertFileSrc("kalasset", icon.data)}?type=path" />`;
  }
}

export function isVScrollable<T extends Element>(el: T | null): boolean {
  return el ? el.scrollHeight > el.clientHeight : false;
}
