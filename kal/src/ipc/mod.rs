use std::borrow::Cow;

use serde::Serialize;
use serialize_to_javascript::Options as JsSerializeOptions;
use strum::{AsRefStr, EnumString};
use winit::dpi::LogicalSize;
use wry::http::{Request, Response};
use wry::WebView;

use crate::app::App;
use crate::config::Config;

pub mod response;

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
}

pub fn emit(
    webview: &WebView,
    event: impl AsRef<str>,
    payload: impl Serialize,
) -> anyhow::Result<()> {
    let js_ser_opts = JsSerializeOptions::default();
    let payload_json = serde_json::value::to_raw_value(&payload).unwrap_or_default();
    let payload_js = serialize_to_javascript::Serialized::new(&payload_json, &js_ser_opts);

    let script = format!(
        r#"(function(){{
            window.KAL.ipc.__handler_store['{}'].forEach(handler => {{
                handler({});
            }});
        }})()"#,
        event.as_ref(),
        payload_js,
    );

    webview.evaluate_script(&script).map_err(Into::into)
}

impl App {
    fn resize_main_window_for_items(&self, count: usize) {
        let main_window = self.main_window();

        let items_height = if count == 0 {
            0
        } else {
            let count = std::cmp::min(count, self.config.appearance.max_items as usize) as u32;
            let item_height = self.config.appearance.item_height + self.config.appearance.item_gap;
            self.config.appearance.input_items_gap + count * item_height
        };

        let height = self.config.appearance.input_height + items_height + Self::MAGIC_BORDERS;

        let size = LogicalSize::new(self.config.appearance.window_width, height);
        let _ = main_window.window().request_surface_size(size.into());
    }

    pub fn ipc_event<'a>(
        &mut self,
        request: Request<Vec<u8>>,
    ) -> anyhow::Result<Response<Cow<'a, [u8]>>> {
        let action: IpcAction = request.uri().path()[1..].try_into()?;

        match action {
            IpcAction::Search => {
                let body = request.body();
                let query = std::str::from_utf8(body)?;

                let mut results = Vec::new();

                let mut store = self.plugin_store.lock();
                store.results(query, &self.fuzzy_matcher, &mut results)?;

                // sort results in reverse so higher scores are first
                results.sort_by(|a, b| b.score.cmp(&a.score));

                let min = std::cmp::min(self.config.general.max_search_results, results.len());
                let final_results = &results[..min];

                self.resize_main_window_for_items(min);
                return response::json(&final_results);
            }

            IpcAction::ClearResults => self.resize_main_window_for_items(0),

            IpcAction::Execute => {
                let payload = request.body();
                let elevated: bool = payload[0] == 1;
                let id = std::str::from_utf8(&payload[1..])?;
                self.plugin_store.execute(id, elevated)?;
                self.hide_main_window(false);
            }

            IpcAction::ShowItemInDir => {
                let id = std::str::from_utf8(request.body())?;
                self.plugin_store.show_item_in_dir(id)?;
                self.hide_main_window(false);
            }

            IpcAction::RefreshIndex => {
                let config = Config::load()?;
                self.plugin_store.refresh(&config)?;
            }

            IpcAction::HideMainWindow => {
                self.hide_main_window(true);
            }
        }

        response::empty()
    }
}
