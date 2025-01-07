use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};

#[derive(PartialEq, Eq, Serialize, Deserialize, EnumString, AsRefStr, Clone, Copy)]
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
