use std::borrow::Cow;
use std::sync::mpsc::{self, Sender};

use serde::Serialize;
use serialize_to_javascript::{Options as JsSerializeOptions, Template as JsTemplate};
use strum::{AsRefStr, EnumString};
use winit::event_loop::EventLoopProxy;
use wry::http::{Method, Request, Response};
use wry::{WebView, WebViewId};

use crate::app::AppEvent;

pub mod response;

pub const INIT_SCRIPT: &str = include_str!("./ipc.js");

#[derive(EnumString, AsRefStr)]
pub enum IpcAction {
    Search,
    ClearResults,
    Execute,
    ShowItemInDir,
    RefreshIndex,
    HideMainWindow,
}

#[derive(EnumString, AsRefStr)]
pub enum IpcEvent {
    FocusInput,
    UpdateConfig,
}

const EMIT_TEMPLATE: &str = r#"(function(){{
    window.KAL.ipc.__handler_store[__TEMPLATE_event__].forEach(handler => {{
        handler(__TEMPLATE_payload__);
    }});
}})()"#;

#[derive(JsTemplate)]
struct EmitScript<'a> {
    event: &'a str,
    payload: &'a serde_json::value::RawValue,
}

pub fn emit(
    webview: &WebView,
    event: impl AsRef<str>,
    payload: impl Serialize,
) -> anyhow::Result<()> {
    let payload = serde_json::value::to_raw_value(&payload)?;

    let emit_script = EmitScript {
        event: event.as_ref(),
        payload: &payload,
    };

    let js_ser_opts = JsSerializeOptions::default();
    let emit_script = emit_script
        .render(EMIT_TEMPLATE, &js_ser_opts)?
        .into_string();

    webview.evaluate_script(&emit_script).map_err(Into::into)
}

pub const PROTOCOL_NAME: &str = "kalipc";

type ProtocolReturn = anyhow::Result<Response<Cow<'static, [u8]>>>;

pub fn make_ipc_protocol(
    sender: Sender<AppEvent>,
    proxy: EventLoopProxy,
) -> impl Fn(WebViewId, Request<Vec<u8>>) -> ProtocolReturn + 'static {
    move |_, request| match *request.method() {
        Method::OPTIONS => self::response::empty(),
        Method::POST => {
            let (tx, rx) = mpsc::sync_channel(1);
            let event = AppEvent::Ipc { request, tx };
            let _ = sender.send(event).inspect_err(|e| tracing::error!("{e}"));
            proxy.wake_up();
            webview2_com::wait_with_pump(rx).unwrap()
        }
        _ => self::response::error("Only POST or OPTIONS method are supported"),
    }
}
