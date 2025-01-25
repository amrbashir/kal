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
    customCSS?: string;

    systemAccentColors: {
      background?: string;
      foreground?: string;
      accent_dark1?: string;
      accent_dark2?: string;
      accent_dark3?: string;
      accent?: string;
      accent_light1?: string;
      accent_light2?: string;
      accent_light3?: string;
      complement?: string;
    };

    config: KalConfig;

    ipc: {
      makeProtocolUrl(protocol: string, path: string): string;
      makeProtocolFileSrc(protocol: string, filePath: string): string;
      invoke<T>(command: IpcCommand, ...payload: any[]): Promise<T>;
      on<T>(event: IpcEvent, handler: (payload: T) => void);
    };
  };
}
