use std::{error::Error, ffi::c_char};

use commands::{fix::execute_command_fix, generate::execute_command_generate};
use psqlx_sys::{backslashResult, PQExpBuffer, PsqlScanState, PsqlSettings};
use psqlx_utils::{to_c_str, to_rust_string, MetaCommand, Plugin};

mod ai;
mod commands;

// Example meta-command implementation
struct GenerateCommand;

impl MetaCommand for GenerateCommand {
    fn name(&self) -> &str {
        "generate"
    }

    fn execute(
        &self,
        scan_state: PsqlScanState,
        _active_branch: bool,
        query_buf: PQExpBuffer,
        _previous_buf: PQExpBuffer,
        pset: PsqlSettings,
    ) -> Result<backslashResult, Box<dyn Error>> {
        execute_command_generate(scan_state, query_buf, pset)
    }
}

struct FixCommand;

impl MetaCommand for FixCommand {
    fn name(&self) -> &str {
        "fix"
    }

    fn execute(
        &self,
        _scan_state: PsqlScanState,
        _active_branch: bool,
        query_buf: PQExpBuffer,
        previous_buf: PQExpBuffer,
        pset: PsqlSettings,
    ) -> Result<backslashResult, Box<dyn Error>> {
        execute_command_fix(query_buf, previous_buf, pset)
    }
}

// Plugin implementation
pub struct AIPlugin;

impl Plugin for AIPlugin {
    fn name(&self) -> &str {
        "ai"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn meta_commands(&self) -> Vec<Box<dyn MetaCommand>> {
        vec![Box::new(GenerateCommand), Box::new(FixCommand)]
    }
}

pub fn name() -> *const c_char {
    to_c_str(AIPlugin.name())
}

pub fn version() -> *const c_char {
    to_c_str(AIPlugin.version())
}

pub fn meta_commands() -> *const c_char {
    let commands = AIPlugin
        .meta_commands()
        .iter()
        .map(|x| x.name())
        .collect::<Vec<&str>>()
        .join(",");
    to_c_str(&commands)
}

pub fn execute_command(
    cmd: *const c_char,
    scan_state: PsqlScanState,
    active_branch: bool,
    query_buf: PQExpBuffer,
    previous_buf: PQExpBuffer,
    pset: PsqlSettings,
) -> backslashResult {
    let cmd_str = match to_rust_string(cmd) {
        Ok(s) => s,
        Err(_) => return backslashResult::PSQL_CMD_ERROR,
    };

    let result = AIPlugin.execute_command(
        cmd_str.as_str(),
        scan_state,
        active_branch,
        query_buf,
        previous_buf,
        pset,
    );

    result
}
