use muxsocial_lib::greeting::compose_greeting_message;
use wasm_bindgen::prelude::*;

/// The WASM-facing client owned by the MuxsocialWorker. All UI access goes through
/// the MuxsocialClientWasmProxy on the main thread, never through this type directly.
#[wasm_bindgen]
pub struct MuxsocialClientWasm {}

#[wasm_bindgen]
impl MuxsocialClientWasm {
    /// Async from day one so the worker RPC shape never changes when real initialisation arrives.
    pub async fn create_new() -> Result<MuxsocialClientWasm, JsValue> {
        Ok(MuxsocialClientWasm {})
    }

    pub async fn compose_greeting(&self, recipient_name: String) -> String {
        compose_greeting_message(&recipient_name)
    }
}
