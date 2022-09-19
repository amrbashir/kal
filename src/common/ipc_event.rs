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

impl Display for IPCEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IPCEvent::Search => write!(f, "search"),
            IPCEvent::Results => write!(f, "results"),
            IPCEvent::Execute => write!(f, "execute"),
            IPCEvent::OpenLocation => write!(f, "open-location"),
            IPCEvent::ClearResults => write!(f, "clear-results"),
            IPCEvent::FocusInput => write!(f, "focus-input"),
            IPCEvent::HideMainWindow => write!(f, "hide-main-window"),
            IPCEvent::RefreshIndex => write!(f, "refresh-index"),
            IPCEvent::RefreshingIndexFinished => write!(f, "refreshing-index-finished"),
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
            "refresh-index" => IPCEvent::RefreshIndex,
            "refreshing-index-finished" => IPCEvent::RefreshingIndexFinished,
            _ => unreachable!(),
        }
    }
}
