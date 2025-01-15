Object.defineProperty(window, "KAL", {
  value: {
    ipc: {
      makeProtocolUrl(protocol, urlPath) {
        return navigator.userAgent.includes("Windows")
          ? `http://${protocol}.localhost/${urlPath}`
          : `${protocol}://${urlPath}`;
      },

      makeProtocolFileSrc(protocol, filePath) {
        const path = encodeURIComponent(filePath);
        return this.makeProtocolUrl(protocol, path);
      },

      toBytes(payload) {
        switch (typeof payload) {
          case "string":
            return new TextEncoder().encode(payload);
          case "boolean":
            return payload ? [1] : [0];
          default:
            throw new Error(`Unimplemented toBytes type: ${typeof payload}`);
        }
      },

      async invoke(action, ...payload) {
        const url = this.makeProtocolUrl("kalipc", action);

        let buffer = new ArrayBuffer(0);
        let view = new Uint8Array(buffer);
        if (payload) {
          for (const arg of payload) {
            const bytes = this.toBytes(arg);
            const currentLen = buffer.byteLength;
            buffer = buffer.transfer(currentLen + bytes.length);
            view = new Uint8Array(buffer);
            view.set(bytes, currentLen);
          }
        }

        const res = await fetch(url, {
          method: "POST",
          headers: {
            "Content-Type": "application/octet-stream",
          },
          body: view,
        });

        const contentType = res.headers.get("Content-Type");

        if (!contentType) return res.arrayBuffer();

        if (contentType.includes("application/json")) return res.json();
        else if (contentType.includes("text/")) return res.text();
        else return res.arrayBuffer();
      },

      __handler_store: {},
      on: function (event, handler) {
        if (typeof this.__handler_store[event] == "undefined") this.__handler_store[event] = [];
        this.__handler_store[event].push(handler);
      },
    },
  },
});
