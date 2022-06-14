use std::fmt::Display;

use serde::Serialize;
use wry::{application::window::WindowId, webview::WebView};

pub enum AppEvent {
    /// An Ipc event from the webview
    Ipc(WindowId, String),
    /// Describes an event from a [`WebView`]
    WebviewEvent {
        event: WebviewEvent,
        window_id: WindowId,
    },
}

pub enum WebviewEvent {
    /// The webview gained or lost focus
    ///
    /// Currently, it is only used on Windows
    #[cfg(target_os = "windows")]
    Focus(bool),
}

/// Emits an event to a window
///
/// This invokes the js handlers registred through `window.KAL.ipc.on()`
pub fn emit_event(webview: &WebView, event: &str, payload: &impl Serialize) {
    if webview
        .evaluate_script(
            format!(
                r#"
              (function(){{
                window.KAL.ipc.__event_handlers['{}'].forEach(handler => {{
                  handler({});
                }});
              }})()
            "#,
                event,
                serialize_to_javascript::Serialized::new(
                    &serde_json::value::to_raw_value(payload).unwrap_or_default(),
                    &serialize_to_javascript::Options::default()
                ),
            )
            .as_str(),
        )
        .is_err()
    {
        println!("[ERROR][IPC]: failed to emit `{}` event", event);
    };
}

pub const INIT_SCRIPT: &str = r#"
  Object.defineProperty(window, "KAL", {
    value: {
      ipc: {
        send: (eventName, ...payload) => {
          window.ipc.postMessage(`${eventName}::${JSON.stringify(payload)}`);
        },
        __event_handlers: {},
        on: function (eventName, event_handler) {
          if (typeof this.__event_handlers[eventName] == "undefined")
            this.__event_handlers[eventName] = [];
          this.__event_handlers[eventName].push(event_handler);
        },
      },
    },
  });
"#;
