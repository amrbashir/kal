/// <reference types="vite/client" />

interface Window {
  ipc: { postMessage(msg: string): void };
  KAL: {
    config?: {
      appearance: {
        transparent?: boolean;
        input_height: number;
        results_row_height: number;
      };
    };

    ipc: {
      send(
        event: import("./ipc_event.ts").IPCEvent,
        ...payload: unknown[]
      ): void;
      on<T>(
        event: import("./ipc_event.ts").IPCEvent,
        eventHandler: (...payload: T[]) => void,
      ): void;
    };
  };
}
