use std::ffi::*;

use serde::Serialize;

use crate::action::{self, Action, CAction};
use crate::icon::Icon;
use crate::CIcon;

/// Represents an item in search results.
#[derive(Serialize, Debug, Clone)]
pub struct ResultItem {
    /// Unique identifier for the result item.
    pub id: String,
    /// Icon associated with this result item.
    pub icon: Icon,
    /// Main text displayed for the result item.
    pub primary_text: String,
    /// Additional descriptive text for the result item.
    pub secondary_text: String,
    /// Optional tooltip text that appears on hover.
    pub tooltip: Option<String>,
    /// List of actions that can be performed on this result item.
    pub actions: Vec<Action>,
    /// Relevance score used for sorting results (higher values indicate higher relevance).
    pub score: u16,
    #[serde(skip)]
    pub c_item: Option<CResultItem>,
}

/// Represents a search result item to be displayed in the user interface.
#[derive(Clone, Debug)]
#[safer_ffi::derive_ReprC]
#[repr(C)]
#[safer_ffi::ffi_export]
pub struct CResultItem {
    /// Unique identifier string for the result item as a C string pointer.
    pub id: *const c_char,
    /// Icon path or identifier for the result as a C string pointer.
    pub icon: CIcon,
    /// Main display text for the result item as a C string pointer.
    pub primary_text: *const c_char,
    /// Supplementary description text as a C string pointer.
    pub secondary_text: *const c_char,
    /// Optional hover text displayed when the user mouses over the item.
    /// A null pointer indicates no tooltip.
    pub tooltip: *const c_char,
    /// Pointer to an array of actions that can be performed on this result item.
    pub actions: *const CAction,
    /// Length of the actions array.
    pub actions_len: usize,
    /// Relevance score for the result item (higher values indicate greater relevance).
    pub score: u16,
}

impl From<CResultItem> for ResultItem {
    fn from(c_item: CResultItem) -> Self {
        let id = unsafe { std::ffi::CStr::from_ptr(c_item.id) }
            .to_string_lossy()
            .into_owned();
        let primary_text = unsafe { std::ffi::CStr::from_ptr(c_item.primary_text) }
            .to_string_lossy()
            .into_owned();
        let secondary_text = unsafe { std::ffi::CStr::from_ptr(c_item.secondary_text) }
            .to_string_lossy()
            .into_owned();
        let tooltip = if c_item.tooltip.is_null() {
            None
        } else {
            Some(
                unsafe { std::ffi::CStr::from_ptr(c_item.tooltip) }
                    .to_string_lossy()
                    .into_owned(),
            )
        };

        let actions = unsafe {
            std::slice::from_raw_parts(c_item.actions, c_item.actions_len)
                .into_iter()
                .map(|action| Action::from(action))
                .collect::<Vec<_>>()
        };

        unsafe {
            ResultItem {
                id,
                icon: c_item.icon.into(),
                primary_text,
                secondary_text,
                tooltip,
                actions,
                score: c_item.score,
                c_item: Some(c_item),
            }
        }
    }
}
