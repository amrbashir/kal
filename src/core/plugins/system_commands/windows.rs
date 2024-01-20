use std::process::Command;

use windows_sys::Win32::System::Power::SetSuspendState;

use crate::common::SearchResultItem;

use super::SystemCommand;

impl SystemCommand {
    const fn shutdown_bin_args(&self) -> &[&str] {
        match self {
            Self::Shutdown => &["/s", "/t", "5000"],
            Self::Restart => &["/r", "/t", "5000"],
            Self::SignOut => &["/l"],
            Self::Hibernate => &["/h"],
            Self::Sleep => unreachable!(),
        }
    }

    fn execute(&self) {
        match self {
            SystemCommand::Sleep => {
                unsafe { SetSuspendState(0, 0, 0) };
            }
            _ => {
                let shutdown_bin = std::env::var("SYSTEMROOT").map_or_else(
                    |_| "shutdown.exe".to_string(),
                    |p| format!("{p}\\System32\\shutdown.exe"),
                );
                if let Err(e) = Command::new(shutdown_bin)
                    .args(self.shutdown_bin_args())
                    .spawn()
                {
                    tracing::error!("Failed to spawn shutdown.exe: {e}");
                }
            }
        }
    }
}

pub fn execute(item: &SearchResultItem) {
    let command: SystemCommand = match item.execution_args.as_str().unwrap().parse() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("{}", e);
            return;
        }
    };

    command.execute()
}
