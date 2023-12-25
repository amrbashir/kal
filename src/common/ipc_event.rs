use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub enum IPCEvent {
    Search,
    Results,
    Execute,
    OpenLocation,
    ClearResults,
    FocusInput,
    HideMainWindow,
    RefreshIndex,
    RefreshingIndexFinished,
}

impl AsRef<str> for IPCEvent {
    fn as_ref(&self) -> &str {
        match self {
            IPCEvent::Search => "search",
            IPCEvent::Results => "results",
            IPCEvent::Execute => "execute",
            IPCEvent::OpenLocation => "open-location",
            IPCEvent::ClearResults => "clear-results",
            IPCEvent::FocusInput => "focus-input",
            IPCEvent::HideMainWindow => "hide-main-window",
            IPCEvent::RefreshIndex => "refresh-index",
            IPCEvent::RefreshingIndexFinished => "refreshing-index-finished",
        }
    }
}

impl Display for IPCEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
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
            "refresh-index" => IPCEvent::RefreshIndex,
            "refreshing-index-finished" => IPCEvent::RefreshingIndexFinished,
            _ => unreachable!(),
        }
    }
}
