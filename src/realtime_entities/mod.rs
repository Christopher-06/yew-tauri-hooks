use serde::Serialize;
use std::{collections::HashSet, sync::Arc};
use tauri::Manager;

mod rw_lock;

pub struct LiveMaster {
    app_handle: tauri::AppHandle,
    known_object_ids: Arc<std::sync::RwLock<HashSet<String>>>,
}

impl LiveMaster {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        let this = Self {
            app_handle,
            known_object_ids: Arc::new(std::sync::RwLock::new(HashSet::new())),
        };
        
        this.register_object_discovery();
        this
    }

    fn register_object_discovery(&self) {
        let known_object_ids = self.known_object_ids.clone();
        let app_handle = self.app_handle.clone();

        let handler = {
            let app_handle = self.app_handle.clone();

            move |_| {
                let known_object_ids = known_object_ids.read().unwrap();
                let known_object_ids: Vec<String> = known_object_ids.iter().cloned().collect();

                let _ = app_handle.emit_all(
                    crate::event_names::OBJECT_DISCOVERY_RESPONSE,
                    &known_object_ids,
                );
            }
        };

        app_handle.listen_global(crate::event_names::OBJECT_DISCOVERY_REQUEST, handler);
    }

    pub fn register_rw_locked<U: 'static + Serialize + Send + Sync>(
        &mut self,
        value: U,
    ) -> Arc<rw_lock::LiveRWLock<U>> {
        // Add to known objects
        let object_id = crate::event_names::object_id::<U>();
        self.known_object_ids.write().unwrap().insert(object_id);

        // Create and return the RW Lock
        rw_lock::LiveRWLock::new(value, self.app_handle.clone())
    }
}
