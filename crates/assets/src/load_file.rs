use futures::channel::oneshot;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::future::Future;

pub(crate) struct FileLoader {
    thread_pool: ThreadPool,
}

impl FileLoader {
    pub fn new() -> Result<Self, String> {
        let thread_pool = ThreadPoolBuilder::default()
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { thread_pool })
    }

    pub fn load_file(&self, path: &str) -> impl Future<Output = Result<Vec<u8>, String>> {
        let (tx, rx) = oneshot::channel();

        let path = path.to_owned();
        self.thread_pool.spawn(move || {
            let read_result = std::fs::read(&path);
            let _ = tx.send(read_result.map(|v| v.into()).map_err(|e| e.to_string()));
        });

        async move {
            rx.await
                .unwrap_or_else(|_| Err("The channel was dropped.".to_string()))
        }
    }
}
