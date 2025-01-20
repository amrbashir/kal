import { onMounted, ref } from "vue";
import { IpcEvent } from "../ipc";

export function useSystemAccentColors() {
  const systemAccentColors = ref(window.KAL.systemAccentColors);

  onMounted(() =>
    window.KAL.ipc.on<typeof window.KAL.systemAccentColors>(
      IpcEvent.UpdateSystemAccentColor,
      (newColor) => {
        systemAccentColors.value = newColor;
      },
    ),
  );

  return systemAccentColors;
}
