use std::borrow::Cow;

use global_hotkey::hotkey::HotKey;
use serialize_to_javascript::{Options as JsSerializeOptions, Template as JsTemplate};
use winit::dpi::LogicalSize;
use winit::event_loop::ActiveEventLoop;
use wry::http::{Request, Response};

use crate::app::App;
use crate::config::Config;
use crate::ipc::{response, IpcCommand, IpcEvent};
use crate::webview_window::{WebViewWindow, WebViewWindowBuilder};

const INIT_TEMPLATE: &str = r#"(function () {
  window.KAL.config = __RAW_config__;

  let custom_css = __TEMPLATE_custom_css__;
  if (custom_css) {
    window.addEventListener("DOMContentLoaded", () => {
      const style = document.createElement("style");
      style.textContent = custom_css;
      const head = document.head ?? document.querySelector("head") ?? document.body;
      head.appendChild(style);
    });
  }
})();"#;

#[derive(JsTemplate)]
struct InitScript<'a> {
    #[raw]
    config: &'a serde_json::value::RawValue,
    custom_css: Option<&'a str>,
}

impl App {
    const MAIN_WINDOW_LABEL: &str = "main";

    /// Magic number accounting for top and bottom border
    /// for undecorated window with shadows
    pub const MAGIC_BORDERS: u32 = 2;

    #[cfg(debug_assertions)]
    const MAIN_WINDOW_URL: &str = "http://localhost:9010/";
    #[cfg(not(debug_assertions))]
    const MAIN_WINDOW_URL: &str = "kal://localhost/";

    pub fn create_main_window(&mut self, event_loop: &dyn ActiveEventLoop) -> anyhow::Result<()> {
        let config = serde_json::value::to_raw_value(&self.config)?;

        let custom_css = self
            .config
            .appearance
            .custom_css_file
            .as_ref()
            .map(std::fs::read_to_string)
            .transpose()?;

        let js_ser_opts = JsSerializeOptions::default();
        let init_script = InitScript {
            config: &config,
            custom_css: custom_css.as_deref(),
        }
        .render(INIT_TEMPLATE, &js_ser_opts)?;

        let sender = self.sender.clone();
        let proxy = self.event_loop_proxy.clone();

        let builder = WebViewWindowBuilder::new()
            .url(Self::MAIN_WINDOW_URL)
            .init_script(&init_script.into_string())
            .ipc(Self::MAIN_WINDOW_LABEL, sender, proxy, ipc_handler)
            .inner_size(LogicalSize::new(
                self.config.appearance.window_width,
                self.config.appearance.input_height + Self::MAGIC_BORDERS,
            ))
            .center(true)
            .decorations(false)
            .resizable(false)
            .visible(false)
            .vibrancy(self.config.appearance.vibrancy)
            .transparent(self.config.appearance.transparent)
            .skip_taskbar(cfg!(any(windows, target_os = "linux")))
            .devtools(true);

        let window = builder.build(event_loop)?;

        #[cfg(windows)]
        window.set_dwmwa_transitions(false);

        self.windows.insert(Self::MAIN_WINDOW_LABEL, window);

        Ok(())
    }

    pub fn main_window(&self) -> &WebViewWindow {
        self.windows.get(Self::MAIN_WINDOW_LABEL).as_ref().unwrap()
    }

    pub fn show_main_window(&mut self) -> anyhow::Result<()> {
        #[cfg(windows)]
        self.store_foreground_hwnd();

        let main_window = self.main_window();
        main_window.window().set_visible(true);
        main_window.window().focus_window();
        main_window.emit(IpcEvent::FocusInput, ())
    }

    pub fn hide_main_window(&self, #[allow(unused)] restore_focus: bool) {
        self.main_window().window().set_visible(false);

        #[cfg(windows)]
        if restore_focus {
            self.restore_prev_foreground_hwnd();
        }
    }

    fn resize_main_window_for_items(&self, count: usize) {
        let main_window = self.main_window();

        let items_height = if count == 0 {
            0
        } else {
            let count = std::cmp::min(count, self.config.appearance.max_items as usize) as u32;
            let item_height = self.config.appearance.item_height + self.config.appearance.item_gap;
            (self.config.appearance.input_items_gap * 2) + count * item_height
        };

        let height = self.config.appearance.input_height + items_height + Self::MAGIC_BORDERS;

        let size = LogicalSize::new(self.config.appearance.window_width, height);
        let _ = main_window.window().request_surface_size(size.into());
    }

    fn main_window_ipc_handler<'a>(
        &mut self,
        request: Request<Vec<u8>>,
    ) -> anyhow::Result<Response<Cow<'a, [u8]>>> {
        let command: IpcCommand = request.uri().path()[1..].try_into()?;

        match command {
            IpcCommand::Query => {
                let body = request.body();
                let query = std::str::from_utf8(body)?;

                let mut results = Vec::new();

                self.plugin_store
                    .query(query, &self.fuzzy_matcher, &mut results)?;

                // sort results in reverse so higher scores are first
                results.sort_by(|a, b| b.score.cmp(&a.score));

                let min = std::cmp::min(self.config.general.max_results, results.len());
                let final_results = &results[..min];

                let json = response::json(&final_results);

                self.resize_main_window_for_items(min);

                self.results = results;

                return json;
            }

            IpcCommand::ClearResults => self.resize_main_window_for_items(0),

            IpcCommand::RunAction => {
                let payload = request.body();

                let Some((action, id)) = std::str::from_utf8(payload)?.split_once('#') else {
                    anyhow::bail!("Invalid payload for command `{command}`: {payload:?}");
                };

                let Some(item) = self.results.iter().find(|r| r.id == id) else {
                    anyhow::bail!("Couldn't find result item with this id: {id}");
                };

                let Some(action) = item.actions.iter().find(|a| a.id == action) else {
                    anyhow::bail!("Couldn't find secondary action: {action}");
                };

                action.run(item)?;

                self.hide_main_window(false);
            }

            IpcCommand::Reload => {
                let old_hotkey = self.config.general.hotkey.clone();
                self.config = Config::load()?;

                self.plugin_store.reload(&self.config)?;

                let main_window = self.main_window();
                main_window.emit(IpcEvent::UpdateConfig, &self.config)?;

                let old_hotkey = HotKey::try_from(old_hotkey.as_str())?;
                let new_hotkey = HotKey::try_from(self.config.general.hotkey.as_str())?;
                if old_hotkey != new_hotkey {
                    self.global_hotkey_manager.unregister(old_hotkey)?;
                    self.global_hotkey_manager.register(new_hotkey)?;
                }
            }

            IpcCommand::HideMainWindow => {
                self.hide_main_window(true);
            }
        }

        response::empty()
    }
}

#[inline]
pub fn ipc_handler<'a>(
    app: &mut App,
    request: Request<Vec<u8>>,
) -> anyhow::Result<Response<Cow<'a, [u8]>>> {
    app.main_window_ipc_handler(request)
}
