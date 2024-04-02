use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;
use tauri_sys::tauri;
use yew::platform::spawn_local;
use yew::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub enum CommandState<T> {
    Loading,
    Data(T),
    Error(tauri_sys::Error),
}

impl<T> CommandState<T> {
    pub fn is_loading(&self) -> bool {
        matches!(self, CommandState::Loading)
    }

    pub fn is_data(&self) -> bool {
        matches!(self, CommandState::Data(_))
    }

    pub fn is_error(&self) -> bool {
        matches!(self, CommandState::Error(_))
    }

    /// Unwrap the data from the CommandState or panic if it is not Data
    pub fn unwrap(&self) -> &T {
        match self {
            CommandState::Data(data) => data,
            _ => panic!("CommandState is not Data"),
        }
    }

    pub fn as_ref(&self) -> CommandState<&T> {
        match self {
            CommandState::Loading => CommandState::Loading,
            CommandState::Data(data) => CommandState::Data(data),
            CommandState::Error(e) => CommandState::Error(e.clone()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct UseCommandSettings {
    pub revalidation_interval: Duration,
}

#[hook]
pub fn use_command<R, A>(cmd: String, args: A, settings: UseCommandSettings) -> CommandState<R>
where
    A: 'static + Serialize,
    R: 'static + DeserializeOwned + Clone,
{
    use gloo_timers::future::TimeoutFuture;

    let cmd_state = use_state(|| CommandState::Loading);

    // Run Command in Async Task
    {
        let cmd_state = cmd_state.clone();

        use_effect_with(1, move |_| {
            spawn_local(async move {
                loop {
                    // Update State with Command Result
                    match tauri::invoke(cmd.as_str(), &args).await {
                        Ok(data) => cmd_state.set(CommandState::Data(data)),
                        Err(e) => cmd_state.set(CommandState::Error(e)),
                    }

                    // Revalidate
                    if settings.revalidation_interval.as_millis() > 0 {
                        // Wait for Interval
                        assert!(settings.revalidation_interval.as_millis() < u32::MAX as u128, "Revalidation Interval too large. Milliseconds must be less than u32::MAX");
                        TimeoutFuture::new(settings.revalidation_interval.as_millis() as u32).await;
                    } else {
                        // Disabled
                        break;
                    }
                }
            });
        });
    }

    (*cmd_state).clone()
}
