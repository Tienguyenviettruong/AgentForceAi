use std::sync::OnceLock;

/// Fallback runtime for provider calls made outside the application's Tokio
/// context. One shared pool avoids a scheduler and worker threads per provider.
pub(super) fn shared_runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("agentforge-provider")
            .build()
            .expect("failed to create shared provider runtime")
    })
}

/// Small bounded mailbox for provider output. Network/process readers pause
/// when the UI consumer falls behind instead of growing memory without limit.
pub(super) fn stream_channel<T>() -> (async_channel::Sender<T>, async_channel::Receiver<T>) {
    async_channel::bounded(64)
}
