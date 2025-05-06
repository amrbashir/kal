use std::sync::{mpsc, Arc};

use global_hotkey::hotkey::HotKey;
use kal_config::Config;
use kal_plugin::ResultItem;
use serialize_to_javascript::{Options as JsSerializeOptions, Template as JsTemplate};
use smol::lock::RwLock;
use winit::dpi::LogicalSize;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use wry::http::Request;

use crate::app::{App, AppMessage};
use crate::icon;
use crate::ipc::{response, AsyncIpcMessage, IpcCommand, IpcEvent, IpcResult};
use crate::plugin_manager::PluginManager;
use crate::webview_window::{WebViewWindow, WebViewWindowBuilder};

const INIT_TEMPLATE: &str = r#"(function () {
  window.KAL.config = __RAW_config__;
  window.KAL.customCSS = __TEMPLATE_custom_css__;
})();"#;

#[derive(JsTemplate)]
struct InitScript<'a> {
    #[raw]
    config: &'a serde_json::value::RawValue,
    custom_css: Option<&'a str>,
}

impl App {
    pub fn create_main_window(&mut self, event_loop: &dyn ActiveEventLoop) -> anyhow::Result<()> {
        let span = tracing::debug_span!("app::create::main_window");
        let _enter = span.enter();

        let config_json = serde_json::value::to_raw_value(&self.config)?;

        let custom_css = self
            .config
            .appearance
            .custom_css_file
            .as_ref()
            .map(std::fs::read_to_string)
            .transpose()
            .unwrap_or_default();

        let js_ser_opts = JsSerializeOptions::default();
        let init_script = InitScript {
            config: &config_json,
            custom_css: custom_css.as_deref(),
        }
        .render(INIT_TEMPLATE, &js_ser_opts)?;

        let async_ipc_sender = MainWindowState::spawn(
            self.config.clone(),
            self.sender.clone(),
            self.event_loop_proxy.clone(),
        );

        let icon_service = self.icon_service.clone();

        let builder = WebViewWindowBuilder::new(&mut self.web_context)
            .with_webview_id(MainWindowState::ID)
            .url(MainWindowState::URL)
            .init_script(&init_script.into_string())
            .inner_size(LogicalSize::new(
                self.config.appearance.window_width,
                self.config.appearance.input_height + WebViewWindow::MAGIC_BORDERS,
            ))
            .async_ipc(async_ipc_sender)
            .async_protocol(icon::Service::PROTOCOL_NAME, move |webview_id, request| {
                icon_service.clone().protocol(webview_id, request)
            })
            .center(true)
            .decorations(false)
            .resizable(false)
            .visible(false)
            .vibrancy(self.config.appearance.vibrancy)
            .transparent(self.config.appearance.transparent)
            .skip_taskbar(cfg!(any(windows, target_os = "linux")))
            .devtools(true);

        let window = builder.build(event_loop, &self.sender)?;

        #[cfg(windows)]
        window.set_dwmwa_transitions(false);

        self.windows.insert(MainWindowState::ID, window);

        Ok(())
    }
}

#[derive(Debug)]
enum MainWindowMessage {
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

    config: RwLock<Config>,
    plugin_manager: RwLock<PluginManager>,
    results: RwLock<Vec<ResultItem>>,
}

impl MainWindowState {
    pub const ID: &str = "main";

    #[cfg(debug_assertions)]
    const URL: &str = "http://localhost:9010/";
    #[cfg(not(debug_assertions))]
    const URL: &str = "kal://localhost/";

    fn new(
        config: Config,
        main_thread_sender: mpsc::Sender<AppMessage>,
        event_loop_proxy: EventLoopProxy,
    ) -> Self {
        let max_results = config.general.max_results;

        let mut plugin_manager = PluginManager::all(&config);
        plugin_manager.reload(&config);

        Self {
            main_thread_sender,
            event_loop_proxy,
            config: RwLock::new(config),
            plugin_manager: RwLock::new(plugin_manager),
            results: RwLock::new(Vec::with_capacity(max_results)),
        }
    }

    fn spawn(
        config: Config,
        main_thread_sender: mpsc::Sender<AppMessage>,
        event_loop_proxy: EventLoopProxy,
    ) -> smol::channel::Sender<MainWindowMessage> {
        let (sender, receiver) = smol::channel::unbounded();

        let state = MainWindowState::new(config, main_thread_sender, event_loop_proxy);

        smol::spawn(async move {
            let state = Arc::new(state);

            loop {
                if let Ok(task) = receiver.recv().await {
                    match task {
                        MainWindowMessage::Ipc { request, tx } => {
                            let state = state.clone();

                            smol::spawn(async move {
                                let res = state.ipc_handler(request).await;

                                if let Err(e) = tx.send(res).await {
                                    tracing::error!("Failed to send async ipc response: {e}");
                                }
                            })
                            .detach();
                        }
                    }
                }
            }
        })
        .detach();

        sender
    }

    /// Batches an event to main thread but doesn't wake the event loop.
    fn batch_event(&self, event: AppMessage) -> anyhow::Result<()> {
        self.main_thread_sender.send(event).map_err(Into::into)
    }

    /// Wakes the event loop.
    fn wake_event_loop(&self) {
        self.event_loop_proxy.wake_up()
    }

    /// Batches an event to main thread and immediately wakes up the event loop.
    fn send_event(&self, event: AppMessage) -> anyhow::Result<()> {
        self.batch_event(event)?;
        self.wake_event_loop();
        Ok(())
    }

    fn resize_main_window_for_items(&self, config: &Config, count: usize) -> anyhow::Result<()> {
        let items_height = if count == 0 {
            0
        } else {
            let count = std::cmp::min(count, config.appearance.max_items as usize) as u32;
            let item_height = config.appearance.item_height + config.appearance.item_gap;
            (config.appearance.input_items_gap * 2) + count * item_height
        };

        let size = LogicalSize::new(
            config.appearance.window_width,
            config.appearance.input_height + items_height + WebViewWindow::MAGIC_BORDERS,
        );

        self.send_event(AppMessage::RequestSufaceSize(size.into()))
    }

    async fn ipc_handler(&self, request: Request<Vec<u8>>) -> IpcResult {
        let span = tracing::debug_span!("ipc::handle::request", ?request);
        let _enter = span.enter();

        let ipc_command: IpcCommand = request.uri().path()[1..].try_into()?;

        span.record("ipc_command", ipc_command.as_ref());

        match ipc_command {
            IpcCommand::Query => {
                let body = request.body();
                let query = std::str::from_utf8(body)?;

                // it is fine to block here since only one query can be processed at a time
                let mut plugins_store = self.plugin_manager.write().await;

                let results = plugins_store.query(query)?;

                let config = self.config.read().await;

                let min = std::cmp::min(config.general.max_results, results.len());
                let final_results = &results[..min];

                let json = response::json(&final_results);

                self.resize_main_window_for_items(&config, min)?;

                *self.results.write().await = results;

                return json;
            }

            IpcCommand::ClearResults => {
                let config = self.config.read().await;
                self.resize_main_window_for_items(&config, 0)?
            }

            IpcCommand::RunAction => {
                let payload = request.body();

                let Some((action, id)) = std::str::from_utf8(payload)?.split_once('#') else {
                    anyhow::bail!("Invalid payload for command `{ipc_command}`: {payload:?}");
                };

                let results = self.results.read().await;

                let Some(item) = results.iter().find(|r| r.id == id) else {
                    anyhow::bail!("Couldn't find result item with this id: {id}");
                };

                let Some(action) = item.actions.iter().find(|a| a.id == action) else {
                    anyhow::bail!("Couldn't find secondary action: {action}");
                };

                action.run(item)?;

                self.send_event(AppMessage::HideMainWindow(false))?;
            }

            IpcCommand::Reload => {
                let mut config = self.config.write().await;

                let old_hotkey = config.general.hotkey.clone();

                *config = Config::load_with_fallback();

                let mut plugin_manager = self.plugin_manager.write().await;
                plugin_manager.reload(&config);

                let old_hotkey = HotKey::try_from(old_hotkey.as_str())?;
                let new_hotkey = HotKey::try_from(config.general.hotkey.as_str())?;
                if old_hotkey != new_hotkey {
                    self.batch_event(AppMessage::ReRegisterHotKey(old_hotkey, new_hotkey))?;
                }

                let json_config = serde_json::to_value(&*config)?;
                let event = AppMessage::MainWindowEmit(IpcEvent::UpdateConfig, json_config);
                self.batch_event(event)?;

                let custom_css = match config.appearance.custom_css_file.as_ref() {
                    Some(path) => smol::fs::read_to_string(path)
                        .await
                        .map(serde_json::Value::String)
                        .unwrap_or(serde_json::Value::Null),

                    None => serde_json::Value::Null,
                };
                let event = AppMessage::MainWindowEmit(IpcEvent::UpdateCustomCSS, custom_css);
                self.batch_event(event)?;
                self.wake_event_loop();
            }

            IpcCommand::HideMainWindow => self.send_event(AppMessage::HideMainWindow(false))?,
        }

        response::empty()
    }
}
