use serde::ser::Serialize;
use std::{cell::RefCell, collections::HashMap};
use wry::{
    application::{
        dpi::{LogicalPosition, LogicalSize},
        event::Event,
        event_loop::EventLoop,
        platform::windows::WindowBuilderExtWindows,
        window::WindowBuilder,
        window::{Window, WindowId},
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
  static WEBVIEWS: RefCell< HashMap<WindowId, WebView>> = RefCell::new(HashMap::new());
}

fn ipc_callback(_window: &Window, request: String) {
    if request.starts_with("[IPC::search]") {
        let query = request.replace("[IPC::search]", "");
        emit_event(
            "search-results",
            &vec![
                "next item is the query",
                query.as_str(),
                "previous item is the query",
            ],
        );
    }
}

fn emit_event(event: &str, payload: &impl Serialize) {
    WEBVIEWS.with(|webviews| {
        let webviews = webviews.borrow();
        for (_, wv) in webviews.iter() {
            let _ = wv.evaluate_script(
                format!(
                    "console.log('[EVENT::{}]{}')",
                    event,
                    serde_json::to_string(payload).unwrap_or_default()
                )
                .as_str(),
            );
        }
    });
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

    let search_input_webview_id = search_input_webivew.window().id();
    let search_results_webview_id = search_results_webivew.window().id();

    WEBVIEWS.with(|webviews| {
        let mut webviews = webviews.borrow_mut();
        webviews.insert(search_input_webview_id, search_input_webivew);
        webviews.insert(search_results_webview_id, search_results_webivew);
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
