#[derive(PartialEq)]
pub enum IPCEvent {
    Search,
    Results,
    Execute,
    SelectNextResult,
    SelectPreviousResult,
    ClearResults,
}

impl From<IPCEvent> for &str {
    fn from(e: IPCEvent) -> Self {
        match e {
            IPCEvent::Search => "search",
            IPCEvent::Results => "results",
            IPCEvent::Execute => "execute",
            IPCEvent::SelectNextResult => "select-next-result",
            IPCEvent::SelectPreviousResult => "select-previous-result",
            IPCEvent::ClearResults => "clear-results",
        }
    }
}

impl From<&str> for IPCEvent {
    fn from(s: &str) -> Self {
        match s {
            "search" => IPCEvent::Search,
            "results" => IPCEvent::Results,
            "execute" => IPCEvent::Execute,
            "select-next-result" => IPCEvent::SelectNextResult,
            "select-previous-result" => IPCEvent::SelectPreviousResult,
            "clear-results" => IPCEvent::ClearResults,
            _ => unreachable!(),
        }
    }
}
