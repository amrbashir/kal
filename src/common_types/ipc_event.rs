#[derive(PartialEq)]
pub enum IPCEvent {
    Search,
    Results,
    Execute,
    OpenLocation,
    ClearResults,
    FocusInput,
    HideMainWindow,
}

impl From<IPCEvent> for &str {
    fn from(e: IPCEvent) -> Self {
        match e {
            IPCEvent::Search => "search",
            IPCEvent::Results => "results",
            IPCEvent::Execute => "execute",
            IPCEvent::OpenLocation => "open-location",
            IPCEvent::ClearResults => "clear-results",
            IPCEvent::FocusInput => "focus-input",
            IPCEvent::HideMainWindow => "hide-main-window",
        }
    }
}

impl From<&str> for IPCEvent {
    fn from(s: &str) -> Self {
        match s {
            "search" => IPCEvent::Search,
            "results" => IPCEvent::Results,
            "execute" => IPCEvent::Execute,
            "open-location" => IPCEvent::OpenLocation,
            "clear-results" => IPCEvent::ClearResults,
            "focus-input" => IPCEvent::FocusInput,
            "hide-main-window" => IPCEvent::HideMainWindow,
            _ => unreachable!(),
        }
    }
}
