use futures_util::StreamExt;
use gloo_timers::future::TimeoutFuture;
use std::time::Duration;
use tauri_sys::event;
use yew::platform::spawn_local;
use yew::prelude::*;

#[cfg(feature = "console")]
use gloo_console::log;

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum LiveObjectState<T> {
    Loading,
    Data(T),
}

impl<T> LiveObjectState<T> {
    pub fn is_loading(&self) -> bool {
        matches!(self, LiveObjectState::Loading)
    }

    pub fn is_data(&self) -> bool {
        matches!(self, LiveObjectState::Data(_))
    }

    /// Unwrap the data from the LiveObjectState or panic if it is not Data
    pub fn unwrap(&self) -> &T {
        match self {
            LiveObjectState::Data(data) => data,
            _ => panic!("LiveObjectState is not Data"),
        }
    }

}


#[derive(Clone, Debug, PartialEq)]
pub struct LiveObjectHookSettings {
    pub once: bool,
    pub min_update_interval: Duration,
}

impl Default for LiveObjectHookSettings {
    fn default() -> Self {
        Self {
            once: false,
            min_update_interval: Duration::from_secs(0),
        }
    }
}

#[hook]
pub fn use_live_object<T: 'static + PartialEq + for<'de> serde::Deserialize<'de> + Clone>(
    props: LiveObjectHookSettings,
) -> LiveObjectState<T> {
    let sync_obj_state = use_state(|| LiveObjectState::Loading);

    // Listen to changes
    {
        let sync_obj_state = sync_obj_state.clone();
        let channel_name = crate::event_names::changes::<T>();
        let props = props.clone();

        use_effect_with(1, move |_| {
            let (tx, mut rx) = tokio::sync::watch::channel::<Option<T>>(None);

            // Create Timer to check for updates
            spawn_local(async move {
                // Mark first value as seen => none
                rx.mark_unchanged();

                loop {
                    // Check if we need to update
                    rx.changed().await.expect("failed to receive data");
                    match rx.borrow().as_ref() {
                        None => break, // channel closed
                        Some(data) => {
                            sync_obj_state.set(LiveObjectState::Data(data.clone()));
                        }
                    }

                    // do props
                    if props.once {
                        break;
                    }

                    // wait min_update_intervall Duration
                    if props.min_update_interval.as_millis() > 0 {
                        assert!(props.min_update_interval.as_millis() < u32::MAX as u128, "Min Update Interval too large. Milliseconds must be less than u32::MAX");
                        TimeoutFuture::new(props.min_update_interval.as_millis() as u32).await;
                    }
                }
            });

            // Create Channel Listener to listen for changes and receive latest data
            spawn_local(async move {
                let mut change_listener = event::listen::<String>(&channel_name).await;

                if let Ok(listener) = change_listener.as_mut() {
                    while let Some(recv_data) = listener.next().await {
                        let data: T = serde_json::from_str(&recv_data.payload)
                            .expect("failed to deserialize data");

                        if tx.send(Some(data)).is_err() {
                            break; // receiver closed
                        }
                    }

                    let _ = tx.send(None);
                }
            });
        });
    }

    // Ask for inital data once
    {
        let sync_obj_state = sync_obj_state.clone();
        let channel_name = crate::event_names::init::<T>();
        let is_loading = *sync_obj_state == LiveObjectState::Loading;

        use_effect_with(is_loading, move |is_loading| {
            if *is_loading {
                spawn_local(async move {
                    tauri_sys::event::emit(&channel_name, &())
                        .await
                        .expect("failed to emit init event");
                });
            }
        });
    }

    // DEBUG ONLY: Check that the object is available
    #[cfg(debug_assertions)]
    {
        use_effect_with(1, move |_| {
            // wait for object dictionary
            spawn_local(async move {
                let mut obj_dict_listener =
                    event::listen::<Vec<String>>(&crate::event_names::OBJECT_DISCOVERY_RESPONSE).await;
                if let Ok(listener) = obj_dict_listener.as_mut() {
                    let item = listener
                        .next()
                        .await
                        .expect("failed to receive object dictionary");
                   

                    let obj_dict = item.payload;
                    assert!(
                        obj_dict.contains(&crate::event_names::object_id::<T>()),
                        "Object {} not found in object dictionary",
                        std::any::type_name::<T>()
                    );
                }
            });

            // send request
            spawn_local(async move {
                tauri_sys::event::emit(crate::event_names::OBJECT_DISCOVERY_REQUEST, &())
                    .await
                    .expect("failed to emit object discovery request");
            });
        });
    }

    (*sync_obj_state).clone()
}
