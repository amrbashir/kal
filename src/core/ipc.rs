use crate::WEBVIEWS;
use serde::ser::Serialize;
use wry::application::window::Window;

/// The script to inject kal's IPC system on the `window` object
pub const KAL_IPC_SCRIPT: &str = r#"
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

/// Parses the IPC request, sent by a window through `window.KAL.ipc.send()` and executes the callback
pub fn handle_ipc<F>(window: &Window, request: String, callback: F)
where
    F: Fn(&Window, &str, Vec<&str>),
{
    let mut s = request.split("::");
    let event_name = s.next().unwrap_or_default();
    let payload_str = s.next().unwrap_or_default();
    let payload = serde_json::from_str::<Vec<&str>>(payload_str).unwrap_or_default();
    callback(window, event_name, payload);
}

/// Emits an event to a window
///
/// This invokes the handlers registred through `window.KAL.ipc.on()`
pub fn emit_event(window_id: u8, event_name: &str, payload: &impl Serialize) {
    WEBVIEWS.with(|webviews| {
        let webviews = webviews.borrow();
        if let Some(wv) = webviews.get(&window_id) {
            if wv
                .evaluate_script(
                    format!(
                        r#"
                          (function(){{
                            window.KAL.ipc.__event_handlers['{}'].forEach(handler => {{
                              console.log('{}');
                              handler(JSON.parse('{}'));
                            }});
                          }})()
                        "#,
                        event_name,
                        serde_json::to_string(payload).unwrap_or_else(|_| "[]".into()),
                        serde_json::to_string(payload).unwrap_or_else(|_| "[]".into()),
                    )
                    .as_str(),
                )
                .is_err()
            {
                println!("[ERROR][IPC]: failed to emit `{}` event", event_name);
            };
        } else {
            println!("[ERROR][IPC]: Failed to find the window for the event");
        }
    });
}
