use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Add it to a struct to know when all the clones are dropped
#[derive(Clone)]
pub struct DropObserver {
    inner: Arc<InnerSignal>,
}

impl DropObserver {
    // Returns a signal struct to check if the object was dropped
    pub fn signal(&self) -> DropSignal {
        self.inner.flag.clone()
    }
}

impl Default for DropObserver {
    fn default() -> Self {
        Self {
            inner: Arc::new(InnerSignal {
                flag: DropSignal(Arc::new(AtomicBool::new(false))),
            }),
        }
    }
}

#[derive(Clone)]
pub struct DropSignal(Arc<AtomicBool>);

impl DropSignal {
    pub fn is_expired(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

#[derive(Clone)]
struct InnerSignal {
    flag: DropSignal,
}

impl Drop for InnerSignal {
    fn drop(&mut self) {
        self.flag.0.store(true, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_signal_state() {
        let observer = DropObserver::default();
        let signal = observer.signal();
        assert!(
            !signal.is_expired(),
            "Signal should not be expired at creation"
        );
    }

    #[test]
    fn test_drop_observer_single_clone() {
        let observer = DropObserver::default();
        let signal = observer.signal();

        let clone = observer.clone();
        drop(clone);

        assert!(
            !signal.is_expired(),
            "Signal should not be expired after dropping a single clone"
        );
    }

    #[test]
    fn test_drop_observer_multiple_clones() {
        let observer = DropObserver::default();
        let signal = observer.signal();

        let clone1 = observer.clone();
        let clone2 = observer.clone();

        drop(clone1);
        assert!(
            !signal.is_expired(),
            "Signal should not be expired after dropping one clone"
        );

        drop(clone2);
        assert!(
            !signal.is_expired(),
            "Signal should not be expired after dropping two clones"
        );

        drop(observer);
        assert!(
            signal.is_expired(),
            "Signal should expire after dropping the original observer and all clones"
        );
    }

    #[test]
    fn test_signal_observer() {
        let observer = DropObserver::default();
        let signal = observer.signal();

        let clone = observer.clone();
        drop(observer);

        assert!(
            !signal.is_expired(),
            "Signal should not be expired while clone exists"
        );

        drop(clone);
        assert!(
            signal.is_expired(),
            "Signal should expire after dropping all clones"
        );
    }
}
