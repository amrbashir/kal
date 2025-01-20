/// <reference types="vite/client" />

declare type KalConfig = {
  general: { tab_through_context_buttons: boolean };

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
      invoke<T>(command: IpcCommand, ...payload: any[]): Promise<T>;
      on<T>(event: IpcEvent, handler: (payload: T) => void);
    };
  };
}
