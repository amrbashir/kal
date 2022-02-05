use serde::ser::Serialize;
use std::{cell::RefCell, collections::HashMap};
use wry::{
    application::{
        dpi::{LogicalPosition, LogicalSize},
        event::Event,
        event_loop::EventLoop,
        platform::windows::WindowBuilderExtWindows,
        window::Window,
        window::WindowBuilder,
    },
    webview::{WebView, WebViewBuilder},
};

#[cfg(not(debug_assertions))]
use wry::http::{Request, Response};

#[cfg(not(debug_assertions))]
fn custom_protocol_callback(request: &Request) -> wry::Result<Response> {
    println!("{:?}", request);
    Ok(Response::default())
}

thread_local! {
  static WEBVIEWS: RefCell< HashMap<u8, WebView>> = RefCell::new(HashMap::new());
}

const SEARCH_INPUT_WINDOW_ID: u8 = 1;
const SEARCH_RESULTS_WINDOW_ID: u8 = 2;

const KAL_IPC_SCRIPT: &'static str = r#"
      window.KAL = {
        ipc: {
          send: (eventName, ...payload) => {
            window.ipc.postMessage(`${eventName}::${JSON.stringify(payload)}`);
          },
          __event_handlers: {},
          on: function (eventName, event_handler) {
            if (typeof this.__event_handlers[eventName] == 'undefined') this.__event_handlers[eventName] = []
            this.__event_handlers[eventName].push(event_handler);
          }
        }
      }
    "#;

/// Emits an event to a window. It runs the event handlers
/// registred by calling `window.KAL.ipc.on()`
fn emit_event(window_id: u8, event_name: &str, payload: &impl Serialize) {
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
                        serde_json::to_string(payload).unwrap_or("[]".into()),
                        serde_json::to_string(payload).unwrap_or("[]".into()),
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

/// Handles an event sent by a window through `window.KAL.ipc.send()`
fn handle_ui_event(event_name: &str, payload: Vec<&str>) {
    if event_name == "search" {
        emit_event(
            SEARCH_RESULTS_WINDOW_ID,
            "results",
            &vec![
                "next item is the query",
                payload[0],
                "previous item is the query",
            ],
        );
    }
}

fn main() {
    let event_loop = EventLoop::new();

    #[cfg(debug_assertions)]
    let search_input_url = "http://localhost:9010/SearchInput";
    #[cfg(not(debug_assertions))]
    // TODO: add assets inside the binary first then use the custom protocol to serve them
    let search_input_url = "kai://SearchInput";
    #[cfg(debug_assertions)]
    let search_results_url = "http://localhost:9010/SearchResults";
    #[cfg(not(debug_assertions))]
    let search_results_url = "kai://SearchResults";

    let search_input_window = WindowBuilder::new()
        .with_inner_size(LogicalSize::<u32>::new(600, 20))
        .with_position(LogicalPosition::<u32>::new(660, 300))
        .with_decorations(false)
        .with_resizable(false)
        .with_skip_taskbar(true)
        .with_transparent(true)
        .build(&event_loop)
        .unwrap();
    #[allow(unused_mut)]
    let mut search_input_webivew_builder = WebViewBuilder::new(search_input_window)
        .unwrap()
        .with_initialization_script(KAL_IPC_SCRIPT)
        .with_url(search_input_url)
        .unwrap()
        .with_ipc_handler(ipc_callback)
        .with_transparent(true);

    #[cfg(not(debug_assertions))]
    {
        search_input_webivew_builder = search_input_webivew_builder
            .with_custom_protocol("kal".into(), custom_protocol_callback);
    }

    let search_input_webivew = search_input_webivew_builder.build().unwrap();

    let search_results_window = WindowBuilder::new()
        .with_inner_size(LogicalSize::<u32>::new(600, 400))
        .with_position(LogicalPosition::<u32>::new(660, 370))
        .with_decorations(false)
        .with_resizable(false)
        .with_skip_taskbar(true)
        .with_transparent(true)
        .build(&event_loop)
        .unwrap();
    #[allow(unused_mut)]
    let mut search_results_webivew_builder = WebViewBuilder::new(search_results_window)
        .unwrap()
        .with_initialization_script(KAL_IPC_SCRIPT)
        .with_url(search_results_url)
        .unwrap()
        .with_ipc_handler(ipc_callback)
        .with_transparent(true);

    #[cfg(not(debug_assertions))]
    {
        search_results_webivew_builder = search_results_webivew_builder
            .with_custom_protocol("kal".into(), custom_protocol_callback);
    }

    let search_results_webivew = search_results_webivew_builder.build().unwrap();

    WEBVIEWS.with(|webviews| {
        let mut webviews = webviews.borrow_mut();
        webviews.insert(SEARCH_INPUT_WINDOW_ID, search_input_webivew);
        webviews.insert(SEARCH_RESULTS_WINDOW_ID, search_results_webivew);
    });

    event_loop.run(move |event, _event_loop, _control_flow| {
        match event {
            Event::WindowEvent { event, .. } => match event {
                _ => {}
            },
            _ => {}
        }
        {}
    });
}

fn ipc_callback(_window: &Window, request: String) {
    let mut s = request.split("::");
    if let Some(event_name) = s.next() {
        if let Some(payload_str) = s.next() {
            if let Ok(payload) = serde_json::from_str::<Vec<&str>>(payload_str) {
                handle_ui_event(event_name, payload);
            } else {
                println!(
                    "[ERROR][IPC]: failed to parse `payload` from `{}`",
                    payload_str
                );
            }
        } else {
            println!(
                "[ERROR][IPC]: failed to parse `payload_str` from `{}`",
                request
            );
        }
    } else {
        println!(
            "[ERROR][IPC]: failed to parse `event_name` from `{}`",
            request
        );
    }
}
