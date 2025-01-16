/// <reference types="vite/client" />

interface Window {
  KAL: {
    systemAccentColor?: string;

    config: {
      appearance: {
        transparent?: boolean;
        input_height: number;
        item_height: number;
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
