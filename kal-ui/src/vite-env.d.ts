/// <reference types="vite/client" />

declare type KalConfig = {
  appearance: {
    transparent?: boolean;
    input_height: number;
    item_height: number;
  };
};

interface Window {
  KAL: {
    systemAccentColor?: string;

    config: KalConfig;

    ipc: {
      makeProtocolUrl(protocol: string, path: string): string;
      makeProtocolFileSrc(protocol: string, filePath: string): string;
      invoke<T>(action: IpcAction, ...payload: any[]): Promise<T>;
      on<T>(event: IpcEvent, handler: (payload: T) => void);
    };
  };
}
