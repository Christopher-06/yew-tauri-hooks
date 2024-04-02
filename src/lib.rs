#[cfg(feature = "wasm")]
pub mod hooks;

#[cfg(feature = "tauri")]
pub mod realtime_entities;

#[cfg(feature = "tauri")]
pub mod observer;

#[allow(dead_code)]
mod event_names {
    use std::hash::{Hash, Hasher};

    pub(crate) const OBJECT_DISCOVERY_REQUEST : &str = "syncer:discover";
    pub(crate) const OBJECT_DISCOVERY_RESPONSE : &str = "syncer:discover:response";

    pub fn object_id<T>() -> String {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::any::type_name::<T>().hash(&mut hasher);
        hasher.finish().to_string()
    }

    pub fn changes<T>() -> String {
        format!("syncer:change:{}", object_id::<T>())
    }

    pub fn init<T>() -> String {
        format!("syncer:init:{}", object_id::<T>())
    }
}
