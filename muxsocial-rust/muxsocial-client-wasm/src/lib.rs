#![feature(try_blocks)]

pub mod indexed_db_config_storage;
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
            // The third-party social-network client SDKs are chatty at trace/debug;
            // pin them to `warn` so muxsocial's own `sources::*` lines stay readable.
            .level_for("nostr", log::LevelFilter::Warn)
            .level_for("nostr_sdk", log::LevelFilter::Warn)
            .level_for("nostr_relay_pool", log::LevelFilter::Warn)
            .level_for("nostr_database", log::LevelFilter::Warn)
            .level_for("nostr_gossip", log::LevelFilter::Warn)
            .level_for("async_wsocket", log::LevelFilter::Warn)
            .level_for("atrium_api", log::LevelFilter::Warn)
            .level_for("atrium_common", log::LevelFilter::Warn)
            .level_for("atrium_xrpc", log::LevelFilter::Warn)
            .level_for("hashiverse_lib", log::LevelFilter::Warn)
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
