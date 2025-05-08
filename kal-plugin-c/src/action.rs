use std::ffi::*;
use std::sync::Arc;

use safer_ffi::option::{self, TaggedOption};
use serde::Serialize;

use crate::icon::{BuiltinIcon, Icon};
use crate::result_item::ResultItem;
use crate::{CIcon, CResultItem};

type ActionFn = dyn Fn(&ResultItem) -> anyhow::Result<()> + Send + Sync;

/// Represents an action that can be performed by the ResultItem.
#[derive(Serialize, Clone)]
pub struct Action {
    /// A unique identifier for the action.
    pub id: String,
    /// An icon to visually represent the action in UI.
    pub icon: Icon,
    /// An optional description explaining what the action does.
    pub description: Option<String>,
    /// An optional keyboard shortcut to trigger the action.
    /// for example "Ctrl+Shift+P"
    pub accelerator: Option<String>,
    /// The function to execute when the action is triggered.
    /// This field is skipped during serialization.
    #[serde(skip)]
    pub action: Arc<ActionFn>,
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
            icon: BuiltinIcon::BlankFile.icon(),
            description: None,
            accelerator: None,
            action: Arc::new(action),
        }
    }

    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.icon = icon;
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
#[safer_ffi::derive_ReprC]
#[derive(Clone)]
#[repr(C)]
pub struct CAction {
    /// A unique identifier for this action, as a C string pointer
    pub id: *const c_char,
    /// The icon associated with this action, as a C string pointer
    pub icon: CIcon,
    /// Description of what this action does, as a C string pointer
    pub description: *const c_char,
    /// Keyboard shortcut definition for this action, as a C string pointer
    /// for example "Ctrl+Shift+P"
    pub accelerator: *const c_char,
    /// Function pointer that will be executed when this action is triggered
    pub action: safer_ffi::closure::ArcDynFn1<(), *const c_void>,
}

impl From<&CAction> for Action {
    fn from(c_action: &CAction) -> Self {
        let id = unsafe { std::ffi::CStr::from_ptr(c_action.id) }
            .to_string_lossy()
            .into_owned();

        let description = if c_action.description.is_null() {
            None
        } else {
            Some(
                unsafe { std::ffi::CStr::from_ptr(c_action.description) }
                    .to_string_lossy()
                    .into_owned(),
            )
        };

        let accelerator = if c_action.accelerator.is_null() {
            None
        } else {
            Some(
                unsafe { std::ffi::CStr::from_ptr(c_action.accelerator) }
                    .to_string_lossy()
                    .into_owned(),
            )
        };

        let action = c_action.action.clone();

        unsafe {
            Action {
                id,
                icon: c_action.icon.into(),
                description,
                accelerator,
                action: Arc::new(move |item: &ResultItem| {
                    let Some(item) = &item.c_item else {
                        return Err(anyhow::anyhow!(
                            "This is was not contstructed from a C ResultItem "
                        ));
                    };
                    let item = item as *const CResultItem as *const c_void;
                    action.call(item);
                    Ok(())
                }),
            }
        }
    }
}
