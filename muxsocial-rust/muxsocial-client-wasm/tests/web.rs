#![cfg(target_arch = "wasm32")]

use muxsocial_client_wasm::muxsocial_client_wasm::MuxsocialClientWasm;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn compose_greeting_round_trips_through_wasm() {
    let muxsocial_client_wasm_instance: MuxsocialClientWasm = MuxsocialClientWasm::create_new().await.expect("create_new failed");
    let greeting_message: String = muxsocial_client_wasm_instance.compose_greeting("wasm-test".to_string());
    assert_eq!(greeting_message, "Hello, wasm-test! Greetings from muxsocial-lib.");
}
