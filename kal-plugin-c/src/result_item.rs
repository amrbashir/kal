use std::ffi::*;

use serde::Serialize;

use crate::action::{Action, CAction};
use crate::icon::Icon;
use crate::CIcon;

/// Represents an item in search results.
#[derive(Serialize, Debug)]
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
}

/// Represents a search result item to be displayed in the user interface.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct CResultItem {
    /// Unique identifier string for the result item as a C string pointer.
    pub id: *const c_char,
    /// Icon path or identifier for the result as a C string pointer.
    pub icon: *const CIcon,
    /// Main display text for the result item as a C string pointer.
    pub primary_text: *const c_char,
    /// Supplementary description text as a C string pointer.
    pub secondary_text: *const c_char,
    /// Optional hover text displayed when the user mouses over the item.
    /// A null pointer indicates no tooltip.
    pub tooltip: *const c_char,
    /// Pointer to an array of actions that can be performed on this result item.
    pub actions: *const CAction,
    /// Number of elements in the actions array.
    pub actions_len: usize,
    /// Relevance score for the result item (higher values indicate greater relevance).
    pub score: u16,
}

impl From<CResultItem> for ResultItem {
    fn from(c_item: CResultItem) -> Self {
        unsafe {
            // Convert CString pointers to Rust Strings
            let id = CString::from_raw(c_item.id as _)
                .to_string_lossy()
                .into_owned();

            let primary_text = CString::from_raw(c_item.primary_text as _)
                .to_string_lossy()
                .into_owned();
            let secondary_text = CString::from_raw(c_item.secondary_text as _)
                .to_string_lossy()
                .into_owned();

            // Convert optional tooltip
            let tooltip = if c_item.tooltip.is_null() {
                None
            } else {
                Some(
                    CString::from_raw(c_item.tooltip as _)
                        .to_string_lossy()
                        .into_owned(),
                )
            };

            // Convert icon
            let icon = Icon::from(*c_item.icon);

            // Convert actions
            let actions = if c_item.actions.is_null() || c_item.actions_len == 0 {
                Vec::new()
            } else {
                let action_slice = std::slice::from_raw_parts(c_item.actions, c_item.actions_len);
                action_slice.iter().map(|&a| a.into()).collect()
            };

            dbg!(444444);

            ResultItem {
                id,
                icon,
                primary_text,
                secondary_text,
                tooltip,
                actions,
                score: c_item.score,
            }
        }
    }
}

impl From<&ResultItem> for CResultItem {
    fn from(item: &ResultItem) -> Self {
        let id_ptr = CString::new(item.id.clone()).unwrap().into_raw();
        let primary_text_ptr = CString::new(item.primary_text.clone()).unwrap().into_raw();
        let secondary_text_ptr = CString::new(item.secondary_text.clone())
            .unwrap()
            .into_raw();

        let tooltip_ptr = match item.tooltip.clone() {
            Some(text) => CString::new(text).unwrap().into_raw(),
            None => std::ptr::null(),
        };

        let icon = Box::new(CIcon::from(item.icon.clone()));

        // Convert actions to C actions

        let (actions_ptr, actions_len) = if item.actions.is_empty() {
            (std::ptr::null(), 0)
        } else {
            let c_actions: Vec<CAction> = item.actions.iter().map(Into::into).collect();
            let len = c_actions.len();
            let ptr = c_actions.as_ptr();
            std::mem::forget(c_actions);
            (ptr, len)
        };

        dbg!(111111111);

        CResultItem {
            id: id_ptr,
            icon: Box::into_raw(icon),
            primary_text: primary_text_ptr,
            secondary_text: secondary_text_ptr,
            tooltip: tooltip_ptr,
            actions: actions_ptr,
            actions_len: actions_len,
            score: item.score,
        }
    }
}
