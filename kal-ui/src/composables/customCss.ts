import { onMounted, ref } from "vue";
import { IpcEvent } from "../ipc";

export function useCustomCSS() {
  const customCSS = ref(window.KAL.customCSS);

  onMounted(() => {
    window.KAL.ipc.on<string>(IpcEvent.UpdateCustomCSS, (newCustomCSS) => {
      customCSS.value = newCustomCSS;
    });
  });

  return customCSS;
}
