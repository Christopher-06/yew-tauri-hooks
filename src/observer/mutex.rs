use std::sync::Arc;
use tokio::sync::{watch, Mutex};

pub struct ObservedMutexGuard<'a, T: Clone + PartialEq> {
    inner: tokio::sync::MutexGuard<'a, T>,
    change_notifier: Arc<watch::Sender<T>>,
    current: watch::Receiver<T>,
}

impl<'a, T: Clone + PartialEq> ObservedMutexGuard<'a, T> {
    pub fn new(
        inner: tokio::sync::MutexGuard<'a, T>,
        change_notifier: Arc<watch::Sender<T>>,
    ) -> Self {
        let current = change_notifier.subscribe();
        Self {
            inner,
            change_notifier,
            current,
        }
    }

    pub fn observe(&self) -> watch::Receiver<T> {
        self.current.clone()
    }
}

impl<'a, T: Clone + PartialEq> Drop for ObservedMutexGuard<'a, T> {
    fn drop(&mut self) {
        // Check if the value has changed
        if *(self.inner) == *(self.current.borrow()) {
            return;
        }

        let _ = self.change_notifier.send(self.inner.clone());
    }
}

impl<'a, T: Clone + PartialEq> std::ops::Deref for ObservedMutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, T: Clone + PartialEq> std::ops::DerefMut for ObservedMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub struct ObservedMutex<T> {
    inner: Mutex<T>,

    change_notifier: Arc<watch::Sender<T>>,
    change_receiver: watch::Receiver<T>,
}

impl<T: Clone + PartialEq> ObservedMutex<T> {
    pub fn new(value: T) -> Self {
        let (change_notifier, change_receiver) = watch::channel(value.clone());
        Self {
            inner: Mutex::new(value),
            change_notifier: Arc::new(change_notifier),
            change_receiver,
        }
    }

    pub async fn lock(&self) -> ObservedMutexGuard<'_, T> {
        let guard = self.inner.lock().await;
        ObservedMutexGuard::new(guard, self.change_notifier.clone())
    }

    pub fn observe(&self) -> watch::Receiver<T> {
        self.change_receiver.clone()
    }
}
