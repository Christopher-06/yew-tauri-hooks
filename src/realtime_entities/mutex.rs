use serde::Serialize;
use std::{ops::Deref, sync::Arc};
use tauri::Manager;

use crate::observer::mutex::ObservedMutex;

pub struct LiveMutex<T: Serialize + PartialEq> {
    inner: ObservedMutex<T>,
}

impl<T: 'static + Serialize + Send + Sync + PartialEq + Clone> LiveMutex<T> {
    pub fn new(value: T, app_handle: tauri::AppHandle) -> Arc<Self> {
        let this = Arc::new(Self {
            inner: ObservedMutex::new(value),
        });

        Self::register_events(this.clone(), app_handle);
        this
    }

    async fn send_change_event(this: Arc<Self>, app_handle: tauri::AppHandle) {
        let observer = this.inner.observe();

        // Serialize acquired data
        let data = observer.borrow();
        let payload = serde_json::to_string(&data.deref()).unwrap();
        let change_event = crate::event_names::changes::<T>();

        // Send data to the frontend
        if let Err(e) = app_handle.emit_all(&change_event, &payload) {
            eprintln!("Error sending data to frontend: {}", e);
        }
    }

    fn register_events(this: Arc<Self>, app_handle: tauri::AppHandle) {
        // Register Change Event
        {
            let mut recv_notifier = this.observe();
            let this = this.clone();
            let app_handle = app_handle.clone();

            // Listen for changes
            tauri::async_runtime::spawn(async move {
                loop {
                    match recv_notifier.changed().await {
                        Err(_) => break, // Sender has been dropped
                        Ok(_) => {
                            // Retrieve Data, Serialize it and send it to the frontend
                            Self::send_change_event(this.clone(), app_handle.clone()).await;
                        }
                    }
                }
            });
        }

        // Register Initial Data Request
        {
            let this = this.clone();
            let request_event = crate::event_names::init::<T>();
            let inner_app_handle = app_handle.clone();

            // listen for the request event
            app_handle.listen_global(request_event, move |_| {
                let this = this.clone();
                let app_handle = inner_app_handle.clone();

                tauri::async_runtime::spawn(async move {
                    Self::send_change_event(this, app_handle).await;
                });
            });
        }
    }
}

impl<T: Serialize + PartialEq> Deref for LiveMutex<T> {
    type Target = ObservedMutex<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
