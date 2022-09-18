/// <reference types="vite/client" />
/// <reference types="vue/macros-global" />

declare interface Window {
  ipc: { postMessage(msg: string): void };
  KAL: {
    ipc: {
      send<T>(
        event: import("../common/ipc_event").IPCEvent,
        ...payload: T
      ): void;
      on<T>(
        event: import("../common/ipc_event").IPCEvent,
        eventHandler: (...payload: T) => void
      ): void;
    };
  };
}

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}
