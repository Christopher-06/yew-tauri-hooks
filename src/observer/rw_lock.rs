use std::sync::Arc;
use tokio::sync::watch;
use tokio::sync::RwLock;
use tokio::sync::RwLockReadGuard;
use tokio::sync::RwLockWriteGuard;

pub struct ObservedRwLockWriteGuard<'a, T>(RwLockWriteGuard<'a, T>, Arc<watch::Sender<()>>);

impl<'a, T> ObservedRwLockWriteGuard<'a, T> {
    pub fn new(value: RwLockWriteGuard<'a, T>, on_drop: Arc<watch::Sender<()>>) -> Self {
        Self(value, on_drop)
    }
}

/// On drop, send a notification to the observers that value has been changed
impl<'a, T> Drop for ObservedRwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        let _ = self.1.send(());
    }
}

/// Delegate Deref to the inner value for easy access
impl<'a, T> std::ops::Deref for ObservedRwLockWriteGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Delegate DerefMut to the inner value for easy access
impl<'a, T> std::ops::DerefMut for ObservedRwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
pub struct ObservedRWLock<T> {
    inner: RwLock<T>,

    change_notifier: Arc<watch::Sender<()>>,
    change_receiver: watch::Receiver<()>,
}

impl<T> ObservedRWLock<T> {
    pub fn new(value: T) -> Self {
        let (change_notifier, change_receiver) = watch::channel(());
        Self {
            inner: RwLock::new(value),
            change_notifier: Arc::new(change_notifier),
            change_receiver,
        }
    }

    pub async fn write(&self) -> ObservedRwLockWriteGuard<'_, T> {
        let guard = self.inner.write().await;
        ObservedRwLockWriteGuard::new(guard, self.change_notifier.clone())
    }

    pub async fn read(&self) -> RwLockReadGuard<'_, T> {
        self.inner.read().await
    }

    pub fn observe(&self) -> watch::Receiver<()> {
        self.change_receiver.clone()
    }
}
