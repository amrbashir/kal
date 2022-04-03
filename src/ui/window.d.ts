declare namespace ipc {
  function postMessage(msg: string): void;
}

declare namespace KAL {
  namespace ipc {
    function send(eventName, ...payload: unknown[]): void;
    function on(
      eventName: string,
      eventHandler: (...payload: unknown[]) => void
    ): void;
  }
}
