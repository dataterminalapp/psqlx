use env_logger::{Builder, Env};
use log::debug;
use plugins::{initialize_plugins, PLUGIN_REGISTRY};
use psqlx_utils::to_c_str;
use std::env;
use std::os::raw::{c_char, c_int};
use std::sync::{Once, OnceLock};

mod plugins;

use psqlx_sys::{
    backslashResult, backslashResult::PSQL_CMD_UNKNOWN, PQExpBuffer, PsqlScanState, PsqlSettings,
};
use psqlx_utils::to_rust_string;

// Static ONCE to ensure initialization happens only once
static INIT: Once = Once::new();
// Static flag to track initialization success
static INIT_RESULT: OnceLock<c_int> = OnceLock::new();

/// Initialize the logger based on environment variables.
/// Uses PSQLX_LOG to determine the log level.
/// This function is safe to call multiple times; it will only initialize the logger once.
pub fn init_logger() {
    // Get the log level from PSQLX_LOG, defaulting to "info"
    let log_level = env::var("PSQLX_LOG").unwrap_or_else(|_| "info".to_string());

    // Create a custom environment configuration
    let env = Env::default().filter_or("RUST_LOG", &log_level);

    // Initialize the logger with our custom configuration
    Builder::from_env(env).format_timestamp_secs().init();

    debug!("Logger initialized with log level: {}", log_level);
}

// Updated external functions to use the new plugin system
#[unsafe(no_mangle)]
pub extern "C" fn has_command_ext(cmd: *const c_char) -> c_int {
    let cmd = match to_rust_string(cmd) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    INIT.call_once(|| {
        init_logger();
        debug!("Initializing plugins");
        if let Err(_e) = initialize_plugins() {
            debug!("Error initializing plugins: {:?}", _e);
        }
        let _ = INIT_RESULT.set(1);
    });

    match INIT_RESULT.get() {
        Some(value) if *value == 1 => {
            debug!("Processing command: {}", cmd);
            if let Ok(registry) = PLUGIN_REGISTRY.read() {
                for plugin in registry.values() {
                    if plugin.commands.contains(cmd.as_str()) {
                        return 1;
                    }
                }
            }
            0
        }
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn exec_command_ext(
    cmd: *const c_char,
    scan_state: PsqlScanState,
    active_branch: bool,
    query_buf: PQExpBuffer,
    previous_buf: PQExpBuffer,
    pset: PsqlSettings,
) -> backslashResult {
    let cmd = match to_rust_string(cmd) {
        Ok(s) => s,
        Err(_) => return PSQL_CMD_UNKNOWN,
    };
    let cmd = cmd.as_str();

    if let Ok(registry) = PLUGIN_REGISTRY.read() {
        for plugin in registry.values() {
            if plugin.commands.contains(cmd) {
                debug!("Running command: {}", cmd);
                return unsafe {
                    (plugin.execute)(
                        to_c_str(cmd),
                        scan_state,
                        active_branch,
                        query_buf,
                        previous_buf,
                        pset,
                    )
                };
            }
        }
    }
    PSQL_CMD_UNKNOWN
}
