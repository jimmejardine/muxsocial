pub mod muxsocial_client_wasm;

use log::{info, trace};
use wasm_bindgen::prelude::*;

/// Initialise logging and panic hook. Must be called manually once per worker before using the WASM module.
#[wasm_bindgen]
pub fn wasm_init(verbose: bool) {
    // Set up logging
    {
        fern::Dispatch::new()
            .level(log::LevelFilter::Trace) // Default level
            .level_for("wasm_bindgen", log::LevelFilter::Warn)
            .chain(fern::Output::call(console_log::log))
            .apply()
            .expect("Failed to initialize logging");

        if verbose {
            info!("Logging initialized");
        }
    }

    console_error_panic_hook::set_once();
    if verbose {
        trace!("WASM module panic hook set");
    }
}
