use serde::Serialize;
use serialize_to_javascript::{Options as JsSerializeOptions, Template as JsTemplate};
use strum::{AsRefStr, Display, EnumString};
use wry::http::Request;
use wry::WebView;

use crate::webview_window::ProtocolResult;

pub mod response;

pub type IpcResult = ProtocolResult;

pub type AsyncIpcSender = smol::channel::Sender<IpcResult>;
pub type AsyncIpcMessage = (Request<Vec<u8>>, AsyncIpcSender);

pub const INIT_SCRIPT: &str = include_str!("./ipc.js");

#[derive(Display, EnumString, AsRefStr, Debug)]
pub enum IpcCommand {
    Query,
    ClearResults,
    RunAction,
    Reload,
    HideMainWindow,
}

#[derive(EnumString, AsRefStr, Debug)]
pub enum IpcEvent {
    FocusInput,
    UpdateConfig,
    UpdateSystemAccentColor,
    UpdateCustomCSS,
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
    let span = tracing::debug_span!(
        "webview::emit",
        webview_id = webview.id(),
        event = event.as_ref()
    );
    let _enter = span.enter();

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

macro_rules! make_async_ipc_protocol {
    ($T:ident, $async_ipc_sender:ident) => {
        move |_webview_id: &str, request: ::wry::http::Request<::std::vec::Vec<u8>>| {
            let async_ipc_sender = $async_ipc_sender.clone();

            async move {
                match *request.method() {
                    ::wry::http::Method::OPTIONS => $crate::ipc::response::empty(),
                    ::wry::http::Method::POST => {
                        let (tx, rx) = ::smol::channel::bounded(1);
                        let task = $T::from((request, tx));
                        async_ipc_sender.send(task).await?;
                        rx.recv().await?
                    }
                    _ => $crate::ipc::response::error("Only POST or OPTIONS method are supported"),
                }
            }
        }
    };
}

pub(crate) use make_async_ipc_protocol;
