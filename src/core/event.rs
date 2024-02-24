use serde::Serialize;
use tao::window::WindowId;
use wry::WebView;

use crate::config::Config;

#[derive(Debug)]
#[non_exhaustive]
pub enum WebviewEvent {
    /// The webview gained or lost focus
    ///
    /// Currently, it is only used on Windows
    #[cfg(windows)]
    Focus(bool),
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ThreadEvent {
    /// Refreshing plugins index has finished
    RefreshingIndexFinished,
    /// Update config in the app state
    UpdateConfig(Config),
}

#[derive(Debug)]
#[non_exhaustive]
pub enum AppEvent {
    /// An Ipc event from the webview
    Ipc(WindowId, String),
    /// Describes an event from a [`WebView`]
    WebviewEvent {
        event: WebviewEvent,
        window_id: WindowId,
    },
    /// Describes an event from a spawned thread
    ThreadEvent(ThreadEvent),
    /// A HotKey event.
    HotKey(global_hotkey::GlobalHotKeyEvent),
}

pub const KAL_IPC_INIT_SCRIPT: &str = r#"Object.defineProperty(window, "KAL", {
    value: {
      ipc: {
        send: (eventName, ...payload) => {
          window.ipc.postMessage(`${eventName}::${JSON.stringify(payload.length === 1 ? payload[0] : payload)}`);
        },
        __event_handlers: {},
        on: function (eventName, event_handler) {
          if (typeof this.__event_handlers[eventName] == "undefined")
            this.__event_handlers[eventName] = [];
          this.__event_handlers[eventName].push(event_handler);
        },
      },
    },
  });"#;

/// Emits an event to a window
///
/// This invokes the js handlers registred through `window.KAL.ipc.on()`
pub fn emit_event<S: AsRef<str>>(
    webview: &WebView,
    event: S,
    payload: impl Serialize,
) -> anyhow::Result<()> {
    let script = format!(
        r#"(function(){{
      window.KAL.ipc.__event_handlers['{}'].forEach(handler => {{
        handler({});
      }});
    }})()"#,
        event.as_ref(),
        serialize_to_javascript::Serialized::new(
            &serde_json::value::to_raw_value(&payload).unwrap_or_default(),
            &serialize_to_javascript::Options::default()
        ),
    );

    webview.evaluate_script(&script).map_err(Into::into)
}
