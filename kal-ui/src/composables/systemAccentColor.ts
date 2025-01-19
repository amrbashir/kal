import { accentFillRest } from "@fluentui/web-components";
import { onMounted, ref } from "vue";
import { IpcEvent } from "../ipc";

export function useSystemAccentColor() {
  const accentColor = accentFillRest.getValueFor(document.documentElement).toColorString();
  const systemAccentColor = ref<string>(window.KAL.systemAccentColor ?? accentColor);

  onMounted(() =>
    window.KAL.ipc.on<string>(IpcEvent.UpdateSystemAccentColor, (newColor) => {
      systemAccentColor.value = newColor;
    }),
  );

  return systemAccentColor;
}
