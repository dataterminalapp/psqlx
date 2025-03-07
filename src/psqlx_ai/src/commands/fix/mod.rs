use psqlx_sys::{
    backslashResult,
    backslashResult::{PSQL_CMD_ERROR, PSQL_CMD_NEWEDIT, PSQL_CMD_SKIP_LINE},
    PQExpBuffer, PQerrorMessage, PsqlSettings,
};
use psqlx_utils::replace_query_buffer_data;
use psqlx_utils::{ask_yea_or_nay, get_schema_json, pqexpbuffer_to_string, spinner::Spinner};
use ureq::{json, serde_json::Value};

use crate::ai::completion;
use std::{error::Error, ffi::CStr};

/// Executes the "fix" command to generate a fix for a previously encountered error.
///
/// This function retrieves the last error message from the PostgreSQL session, attempts to generate a fix
/// for the associated code using the error message and schema information, and presents the fixed code to the user.
/// The user is then asked whether they want to apply the fix to the query buffer. If the user agrees,
/// the fixed code is added to the query buffer for execution.
///
/// # Returns
/// - `Ok(_backslashResult_PSQL_CMD_NEWEDIT)`: If the user agrees to apply the fix, the fixed code is added
///   to the query buffer, and a new edit result is returned.
/// - `Ok(_backslashResult_PSQL_CMD_SKIP_LINE)`: If the user opts not to apply the fix, this result is returned.
/// - `Ok(_backslashResult_PSQL_CMD_ERROR)`: If any errors occur during processing, such as issues retrieving the
///   error message, generating the fix, or handling the output buffers, an error result is returned.
///
/// # Errors
/// - Returns an error if any part of the function fails, including fetching the error message, generating the fix,
///   or handling the buffers.
pub fn execute_command_fix(
    query_buf: PQExpBuffer,
    previous_buf: PQExpBuffer,
    pset: PsqlSettings,
) -> Result<backslashResult, Box<dyn Error>> {
    let mut spinner = Spinner::start();
    let code_str = pqexpbuffer_to_string(previous_buf)?;
    let error_msg = unsafe { PQerrorMessage(pset.db) };
    let error_msg_c_str = unsafe { CStr::from_ptr(error_msg) };
    let err_msg_str = match error_msg_c_str.to_str() {
        Ok(s) => s,
        Err(_) => {
            spinner.stop();
            return Ok(PSQL_CMD_ERROR);
        }
    };
    let schema_str = &get_schema_json(pset);

    if err_msg_str.trim().is_empty() {
        spinner.stop();
        println!("No previous error found.");
        return Ok(PSQL_CMD_SKIP_LINE);
    }

    // Call the fix function
    match fix_code(&code_str, err_msg_str, schema_str) {
        Ok(fixed_code) => {
            let modified = format!("{}", fixed_code);
            spinner.stop();

            println!("{}", modified);

            match ask_yea_or_nay("Run fix?") {
                Ok(true) => {
                    replace_query_buffer_data(query_buf, modified.as_str());
                    println!();
                    return Ok(PSQL_CMD_NEWEDIT);
                }
                _ => {
                    println!("Fix not applied.");
                    return Ok(PSQL_CMD_SKIP_LINE);
                }
            }
        }
        Err(e) => {
            spinner.stop();
            println!("Error: {}", e);
            return Ok(PSQL_CMD_ERROR);
        }
    }
}

pub fn fix_code(code: &str, error_msg: &str, schema: &str) -> Result<String, Box<dyn Error>> {
    let system_message = format!(
        "
            You are a Distinguished Engineer code assistant for PSQL, the Postgres terminal.
            Generate and return only the exact SQL it will be used as input to run again, do not use markdown and return the code formatted, nothing else.
            Current schema of the database is:
            {:?}
        ", schema);

    let messages: &mut Vec<Value> = &mut Vec::new();
    messages.push(json!({
        "role": "user",
        "content": format!("SQL Code: {:?} Error: {:?}", code, error_msg)
    }));

    completion(messages, &system_message)
}
