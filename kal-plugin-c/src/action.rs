use std::ffi::*;

use serde::Serialize;

use crate::icon::{BuiltinIcon, Icon};
use crate::result_item::ResultItem;
use crate::{CIcon, CResultItem};

type ActionFn = dyn Fn(&ResultItem) -> anyhow::Result<()> + Send + Sync;

/// Represents an action that can be performed by the ResultItem.
#[derive(Serialize)]
pub struct Action {
    /// A unique identifier for the action.
    pub id: String,
    /// An optional icon to visually represent the action in UI.
    pub icon: Option<Icon>,
    /// An optional description explaining what the action does.
    pub description: Option<String>,
    /// An optional keyboard shortcut to trigger the action.
    /// for example "Ctrl+Shift+P"
    pub accelerator: Option<String>,
    /// The function to execute when the action is triggered.
    /// This field is skipped during serialization.
    #[serde(skip)]
    pub action: Box<ActionFn>,
}

impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecondaryAction")
            .field("id", &self.id)
            .field("description", &self.description)
            .field("accelerator", &self.accelerator)
            .field("action", &"<action>")
            .finish()
    }
}

impl Action {
    pub fn new<F>(id: &'static str, action: F) -> Self
    where
        F: Fn(&ResultItem) -> anyhow::Result<()> + 'static + Send + Sync,
    {
        Self {
            id: id.to_string(),
            icon: None,
            description: None,
            accelerator: None,
            action: Box::new(action),
        }
    }

    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn with_description(mut self, description: &'static str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn with_accelerator(mut self, accelerator: &'static str) -> Self {
        self.accelerator = Some(accelerator.to_string());
        self
    }

    pub fn run(&self, item: &ResultItem) -> anyhow::Result<()> {
        (self.action)(item)
    }
}

impl Action {
    pub fn primary<F>(action: F) -> Self
    where
        F: Fn(&ResultItem) -> anyhow::Result<()> + 'static + Send + Sync,
    {
        Self::new("RunPrimary", action).with_accelerator("Enter")
    }

    pub fn open_elevated<F>(action: F) -> Self
    where
        F: Fn(&ResultItem) -> anyhow::Result<()> + 'static + Send + Sync,
    {
        Self::new("RunElevated", action)
            .with_icon(BuiltinIcon::Admin.into())
            .with_description("Run as adminstrator")
            .with_accelerator("Shift+Enter")
    }

    pub fn open_location<F>(action: F) -> Self
    where
        F: Fn(&ResultItem) -> anyhow::Result<()> + 'static + Send + Sync,
    {
        Self::new("OpenLocation", action)
            .with_icon(BuiltinIcon::FolderOpen.into())
            .with_description("Open containing folder")
            .with_accelerator("Ctrl+O")
    }
}

/// Represents an action that can be performed by the ResultItem.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct CAction {
    /// A unique identifier for this action, as a C string pointer
    pub id: *const c_char,
    /// The icon associated with this action, as a C string pointer
    pub icon: *const CIcon,
    /// Description of what this action does, as a C string pointer
    pub description: *const c_char,
    /// Keyboard shortcut definition for this action, as a C string pointer
    /// for example "Ctrl+Shift+P"
    pub accelerator: *const c_char,
    /// Function pointer that will be executed when this action is triggered
    pub action: *const extern "C" fn(*const CResultItem),
}

impl From<&Action> for CAction {
    fn from(action: &Action) -> Self {
        let id = CString::new(action.id.clone()).unwrap().into_raw();

        let icon = action
            .icon
            .clone()
            .map(|icon| {
                let c_icon = icon.into();
                Box::into_raw(Box::new(c_icon)) as *mut CIcon
            })
            .unwrap_or(std::ptr::null_mut());

        let description = action
            .description
            .clone()
            .map(|s| CString::new(s).unwrap().into_raw())
            .unwrap_or(std::ptr::null_mut());

        let accelerator = action
            .accelerator
            .clone()
            .map(|s| CString::new(s).unwrap().into_raw())
            .unwrap_or(std::ptr::null_mut());

        let action = action.action.as_ref();
        let action = Box::new(|item: *const CResultItem| {
            let item = unsafe { *item };
            let item = ResultItem::from(item);
            (action)(&item);
        });

        let action = Box::into_raw(action) as *const extern "C" fn(*const CResultItem);

        CAction {
            id,
            icon,
            description,
            accelerator,
            action,
        }
    }
}

impl From<CAction> for Action {
    fn from(c_action: CAction) -> Self {
        let id = unsafe {
            CString::from_raw(c_action.id as _)
                .to_string_lossy()
                .into_owned()
        };
        let icon = unsafe {
            CString::from_raw(c_action.icon as _)
                .to_string_lossy()
                .into_owned()
        };
        let description = unsafe {
            CString::from_raw(c_action.description as _)
                .to_string_lossy()
                .into_owned()
        };
        let accelerator = unsafe {
            CString::from_raw(c_action.accelerator as _)
                .to_string_lossy()
                .into_owned()
        };
        let action: Box<fn(*const CResultItem)> = unsafe { Box::from_raw(c_action.action as _) };
        let action = move |item: &ResultItem| {
            let c_item = CResultItem::from(item);
            // TODO: error handling
            Ok(action(&c_item))
        };

        Action {
            id,
            icon: Some(Icon::builtin(icon)),
            description: Some(description),
            accelerator: Some(accelerator),
            action: Box::new(action),
        }
    }
}
