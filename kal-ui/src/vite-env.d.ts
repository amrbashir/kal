/// <reference types="vite/client" />

interface Window {
  KAL: {
    config?: {
      appearance: {
        transparent?: boolean;
        input_height: number;
        results_row_height: number;
      };
    };

    ipc: {
      makeProtocolUrl(protocol: string, path: string): string;
      makeProtocolFileSrc(protocol: string, filePath: string): string;
      invoke<T>(action: IpcAction, ...payload: any[]): Promise<T>;
      on<T>(event: IpcEvent, handler: (payload?: T) => void);
    };
  };
}
