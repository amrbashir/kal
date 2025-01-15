use std::sync::mpsc;

use serialize_to_javascript::{Options as JsSerializeOptions, Template as JsTemplate};
use winit::dpi::LogicalSize;
use winit::event_loop::ActiveEventLoop;

use crate::app::{App, AppEvent};
use crate::ipc::IpcEvent;
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

impl App {
    const MAIN_WINDOW_KEY: &str = "main";

    pub fn create_main_window(&mut self, event_loop: &dyn ActiveEventLoop) -> anyhow::Result<()> {
        #[cfg(debug_assertions)]
        let url = "http://localhost:9010";
        #[cfg(not(debug_assertions))]
        let url = "kal://localhost";

        #[derive(JsTemplate)]
        struct InitScript {
            #[raw]
            config: String,
            custom_css: Option<String>,
        }

        let config = serde_json::to_string(&self.config)?;
        let custom_css = self
            .config
            .appearance
            .custom_css_file
            .as_ref()
            .map(std::fs::read_to_string)
            .transpose()?;

        let js_ser_opts = JsSerializeOptions::default();
        let init_script = InitScript { config, custom_css }.render(INIT_TEMPLATE, &js_ser_opts)?;

        let sender = self.sender.clone();
        let proxy = self.event_loop_proxy.clone();

        let builder = WebViewWindowBuilder::new()
            .url(url)
            .init_script(&init_script.into_string())
            .ipc(move |_, request| {
                let (tx, rx) = mpsc::sync_channel(1);
                let _ = sender
                    .send(AppEvent::Ipc { request, tx })
                    .inspect_err(|e| tracing::error!("{e}"));
                proxy.wake_up();

                webview2_com::wait_with_pump(rx).unwrap()
            })
            .inner_size(LogicalSize::new(
                self.config.appearance.window_width,
                self.config.appearance.input_height,
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
        self.windows.insert(Self::MAIN_WINDOW_KEY, window);

        Ok(())
    }

    pub fn main_window(&self) -> &WebViewWindow {
        self.windows.get(Self::MAIN_WINDOW_KEY).as_ref().unwrap()
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
}
