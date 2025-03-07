use libloading::{Library, Symbol};
use log::debug;
use once_cell::sync::Lazy;
use psqlx_sys::{backslashResult, PQExpBuffer, PsqlScanState, PsqlSettings};
use psqlx_utils::to_rust_string;
use std::collections::{HashMap, HashSet};
use std::ffi::c_char;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::{env, fs};

// Type for the plugin's initialization function
type PluginCCharFunction = unsafe fn() -> *const c_char;

type PluginCExecuteFunction = unsafe fn(
    cmd: *const c_char,
    scan_state: PsqlScanState,
    active_branch: bool,
    query_buf: PQExpBuffer,
    previous_buf: PQExpBuffer,
    pset: PsqlSettings,
) -> backslashResult;

// Structure to hold loaded plugin information
pub struct LoadedPlugin {
    pub commands: HashSet<String>,
    pub execute: PluginCExecuteFunction,
}

// Global plugin registry
pub static PLUGIN_REGISTRY: Lazy<RwLock<HashMap<String, LoadedPlugin>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

// Function to get the XDG data directory
fn get_xdg_data_dir() -> PathBuf {
    if let Ok(xdg_data) = env::var("XDG_DATA_HOME") {
        PathBuf::from(xdg_data)
    } else {
        PathBuf::from(env::var("HOME").unwrap_or_default())
            .join(".local")
            .join("share")
    }
}

// Function to get the default plugin directory
fn get_default_plugin_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        // On Windows, use %LOCALAPPDATA%\psqlx\plugins
        if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
            PathBuf::from(local_app_data).join("psqlx").join("plugins")
        } else {
            // Fallback to %USERPROFILE%\AppData\Local\psqlx\plugins
            PathBuf::from(env::var("USERPROFILE").unwrap_or_default())
                .join("AppData")
                .join("Local")
                .join("psqlx")
                .join("plugins")
        }
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let plugin_dirs = vec![get_xdg_data_dir().join("psqlx").join("plugins")];

        plugin_dirs
            .into_iter()
            .find(|dir| dir.exists())
            .unwrap_or_else(|| get_xdg_data_dir().join("psqlx").join("plugins"))
    }
}

pub fn initialize_plugins() -> Result<(), Box<dyn std::error::Error>> {
    let plugin_dir = get_default_plugin_dir();

    debug!("Plugins dir: {:?}", plugin_dir);

    let plugin_manager = PluginManager::new(plugin_dir.clone());
    plugin_manager.init()?;

    Ok(())
}

pub struct PluginManager {
    plugin_dir: PathBuf,
}

impl PluginManager {
    pub fn new(plugin_dir: PathBuf) -> Self {
        Self { plugin_dir }
    }

    pub fn init(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.load_embedded_plugins()?;

        if !self.plugin_dir.exists() {
            fs::create_dir_all(&self.plugin_dir)?;
        }

        self.load_plugins()?;
        Ok(())
    }

    fn load_embedded_plugins(&self) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Loading embedded plugins.");

        // AI plugin
        debug!("Loading AI plugin.");
        let meta_commands_ptr = psqlx_ai::meta_commands();
        let meta_commands = self.extract_meta_commands(meta_commands_ptr)?;
        let execute_f = psqlx_ai::execute_command;

        let loaded_plugin = LoadedPlugin {
            commands: meta_commands,
            execute: execute_f,
        };

        let plugin_name = to_rust_string(psqlx_ai::name())?;

        let mut registry = PLUGIN_REGISTRY.write().unwrap();
        registry.insert(plugin_name.to_string(), loaded_plugin);

        Ok(())
    }

    fn load_plugins(&self) -> Result<(), Box<dyn std::error::Error>> {
        for entry in fs::read_dir(&self.plugin_dir)? {
            let entry = entry?;
            let path = entry.path();

            debug!("Validating plugin: {:?}", path);
            if self.is_valid_plugin(&path) {
                self.load_plugin(&path)?;
            }
        }
        Ok(())
    }

    fn is_valid_plugin(&self, path: &Path) -> bool {
        path.extension().map_or(false, |ext| {
            #[cfg(target_os = "windows")]
            let is_lib = ext == "dll";
            #[cfg(target_os = "linux")]
            let is_lib = ext == "so";
            #[cfg(target_os = "macos")]
            let is_lib = ext == "dylib";

            is_lib
        })
    }

    fn extract_meta_commands(
        &self,
        meta_commands_ptr: *const i8,
    ) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
        let meta_commands = to_rust_string(meta_commands_ptr)?;

        let mut commands = HashSet::new();
        for command in meta_commands.split(",") {
            commands.insert(command.to_string());
        }

        Ok(commands)
    }

    fn load_plugin(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            /*
             * I would love to use a struct rather than getting them one by one but
             * I'm not sure how to do that with libloading.
             */
            let library = match Library::new(path) {
                Ok(lib) => lib,
                Err(e) => {
                    debug!("Failed to load library from {:?}: {:?}", path, e);
                    return Err(e.into()); // Convert the error if needed
                }
            };
            debug!("Loaded library successfully: {:?}", path);

            let plugin_name_f: Symbol<PluginCCharFunction> = library.get(b"name")?;
            let plugin_name = to_rust_string(plugin_name_f())?;

            let meta_commands_f: Symbol<PluginCCharFunction> = library.get(b"meta_commands")?;
            let meta_commands = self.extract_meta_commands(meta_commands_f())?;

            let execute_f: Symbol<PluginCExecuteFunction> = library.get(b"execute_command")?;

            debug!("Loaded plugin: {}", plugin_name);
            debug!("Commands: {:?}", meta_commands);

            let loaded_plugin = LoadedPlugin {
                commands: meta_commands,
                execute: *execute_f,
            };

            let mut registry = PLUGIN_REGISTRY.write().unwrap();
            registry.insert(plugin_name.to_string(), loaded_plugin);
        }
        Ok(())
    }
}
