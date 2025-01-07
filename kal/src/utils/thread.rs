/// Spawns a new thread, returning a [`JoinHandle`](std::thread::JoinHandle) for it
/// and log errors if the closure return and [`Err`].
pub fn spawn<F>(f: F) -> std::thread::JoinHandle<()>
where
    F: FnOnce() -> anyhow::Result<()>,
    F: Send + 'static,
{
    std::thread::spawn(move || {
        if let Err(e) = f() {
            tracing::error!("{e}");
        }
    })
}
