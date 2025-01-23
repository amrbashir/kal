use std::path::PathBuf;
use std::sync::mpsc;

use fuzzy_matcher::skim::SkimMatcherV2;
use global_hotkey::hotkey::HotKey;
use serialize_to_javascript::{Options as JsSerializeOptions, Template as JsTemplate};
use winit::dpi::LogicalSize;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use wry::http::Request;

use crate::app::{App, AppMessage};
use crate::config::Config;
use crate::ipc::{response, AsyncIpcMessage, IpcCommand, IpcEvent, IpcResult};
use crate::plugin_store::PluginStore;
use crate::result_item::ResultItem;
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
    pub fn create_main_window(&mut self, event_loop: &dyn ActiveEventLoop) -> anyhow::Result<()> {
        let config_json = serde_json::value::to_raw_value(&self.config)?;

        let custom_css = self
            .config
            .appearance
            .custom_css_file
            .as_ref()
            .map(std::fs::read_to_string)
            .transpose()?;

        let js_ser_opts = JsSerializeOptions::default();
        let init_script = InitScript {
            config: &config_json,
            custom_css: custom_css.as_deref(),
        }
        .render(INIT_TEMPLATE, &js_ser_opts)?;

        let async_ipc_sender = MainWindowState::spawn(
            self.config.clone(),
            self.data_dir.clone(),
            self.sender.clone(),
            self.event_loop_proxy.clone(),
        );

        let builder = WebViewWindowBuilder::new()
            .url(MainWindowState::URL)
            .init_script(&init_script.into_string())
            .inner_size(LogicalSize::new(
                self.config.appearance.window_width,
                self.config.appearance.input_height + WebViewWindow::MAGIC_BORDERS,
            ))
            .async_ipc(async_ipc_sender)
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

        self.windows.insert(MainWindowState::LABEL, window);

        Ok(())
    }
}

#[derive(Debug)]
pub enum MainWindowMessage {
    Ipc {
        request: wry::http::Request<Vec<u8>>,
        tx: smol::channel::Sender<IpcResult>,
    },
}

impl From<AsyncIpcMessage> for MainWindowMessage {
    fn from(value: AsyncIpcMessage) -> Self {
        Self::Ipc {
            request: value.0,
            tx: value.1,
        }
    }
}

pub struct MainWindowState {
    main_thread_sender: mpsc::Sender<AppMessage>,
    event_loop_proxy: EventLoopProxy,

    pub fuzzy_matcher: SkimMatcherV2,

    pub config: Config,
    pub plugin_store: PluginStore,
    pub results: Vec<ResultItem>,
}

impl MainWindowState {
    pub fn spawn(
        config: Config,
        data_dir: PathBuf,
        main_thread_sender: mpsc::Sender<AppMessage>,
        event_loop_proxy: EventLoopProxy,
    ) -> smol::channel::Sender<MainWindowMessage> {
        let (sender, receiver) = smol::channel::unbounded();

        smol::spawn(async move {
            let mut plugin_store = crate::plugins::all(&config, &data_dir);
            plugin_store.reload(&config).await;

            let mut state = MainWindowState {
                config,
                plugin_store,
                fuzzy_matcher: SkimMatcherV2::default(),
                results: Vec::new(),
                main_thread_sender,
                event_loop_proxy,
            };

            loop {
                if let Ok(task) = receiver.recv().await {
                    match task {
                        MainWindowMessage::Ipc { request, tx } => {
                            tracing::debug!("Handling ipc request...");
                            tracing::trace!("{request:?}");
                            let res = state.ipc_handler(request).await;
                            tracing::debug!("Finished handling ipc request");
                            tracing::trace!("{res:?}");

                            if let Err(e) = tx.send(res).await {
                                tracing::error!("Failed to send async ipc response: {e}");
                            }
                        }
                    }
                }
            }
        })
        .detach();

        sender
    }

    pub const LABEL: &str = "main";

    #[cfg(debug_assertions)]
    pub const URL: &str = "http://localhost:9010/";
    #[cfg(not(debug_assertions))]
    pub const URL: &str = "kal://localhost/";

    fn send_event(&self, event: AppMessage) -> anyhow::Result<()> {
        self.main_thread_sender.send(event)?;
        self.event_loop_proxy.wake_up();
        Ok(())
    }

    fn resize_main_window_for_items(&self, count: usize) -> anyhow::Result<()> {
        let items_height = if count == 0 {
            0
        } else {
            let count = std::cmp::min(count, self.config.appearance.max_items as usize) as u32;
            let item_height = self.config.appearance.item_height + self.config.appearance.item_gap;
            (self.config.appearance.input_items_gap * 2) + count * item_height
        };

        let size = LogicalSize::new(
            self.config.appearance.window_width,
            self.config.appearance.input_height + items_height + WebViewWindow::MAGIC_BORDERS,
        );

        self.send_event(AppMessage::RequestSufaceSize(size.into()))
    }

    pub async fn ipc_handler(&mut self, request: Request<Vec<u8>>) -> IpcResult {
        let command: IpcCommand = request.uri().path()[1..].try_into()?;

        match command {
            IpcCommand::Query => {
                let body = request.body();
                let query = std::str::from_utf8(body)?;

                let mut results = Vec::new();

                self.plugin_store
                    .query(query, &self.fuzzy_matcher, &mut results)
                    .await?;

                // sort results in reverse so higher scores are first
                results.sort_by(|a, b| b.score.cmp(&a.score));

                let min = std::cmp::min(self.config.general.max_results, results.len());
                let final_results = &results[..min];

                let json = response::json(&final_results);

                self.resize_main_window_for_items(min)?;

                self.results = results;

                return json;
            }

            IpcCommand::ClearResults => self.resize_main_window_for_items(0)?,

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

                self.send_event(AppMessage::HideMainWindow(false))?;
            }

            IpcCommand::Reload => {
                let old_hotkey = self.config.general.hotkey.clone();
                self.config = Config::load()?;

                self.plugin_store.reload(&self.config).await;

                let old_hotkey = HotKey::try_from(old_hotkey.as_str())?;
                let new_hotkey = HotKey::try_from(self.config.general.hotkey.as_str())?;
                if old_hotkey != new_hotkey {
                    self.send_event(AppMessage::ReRegisterHotKey(old_hotkey, new_hotkey))?;
                }

                let json_config = serde_json::to_value(&self.config)?;
                let event = AppMessage::MainWindowEmit(IpcEvent::UpdateConfig, json_config);
                self.send_event(event)?;
            }

            IpcCommand::HideMainWindow => self.send_event(AppMessage::HideMainWindow(false))?,
        }

        response::empty()
    }
}
