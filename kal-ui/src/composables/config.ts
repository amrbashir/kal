import { onMounted, ref } from "vue";
import { IpcEvent } from "../ipc";

export function useConfig() {
  const config = ref<KalConfig>(window.KAL.config);

  onMounted(() =>
    window.KAL.ipc.on<KalConfig>(IpcEvent.UpdateConfig, (newConfig) => {
      config.value = newConfig;
    }),
  );

  return config;
}
