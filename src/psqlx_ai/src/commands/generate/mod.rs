use std::{error::Error, ptr::null_mut};

use psqlx_sys::{
    backslashResult,
    backslashResult::{PSQL_CMD_ERROR, PSQL_CMD_NEWEDIT, PSQL_CMD_SKIP_LINE},
    slash_option_type::OT_WHOLE_LINE,
    PQExpBuffer, PsqlScanState, PsqlSettings,
};
use psqlx_utils::{
    ask_additional_instructions, extract_args, get_schema_json, replace_query_buffer_data,
    spinner::Spinner, AdditionalInstructions,
};
use ureq::{json, serde_json};

use crate::ai::completion;

/// Executes the "generate" command for generating code based on schema information.
///
/// This function processes a given command option, generates code based on the schema, and
/// presents the generated code to the user. The user is then asked whether they want to run the generated code.
/// If the user agrees, the generated code is appended to the query buffer for execution.
///
/// # Returns
/// - `Ok(_backslashResult_PSQL_CMD_NEWEDIT)`: If the user agrees to run the generated code, it is added
///   to the query buffer, and the result indicates that a new edit is required.
/// - `Ok(_backslashResult_PSQL_CMD_SKIP_LINE)`: If the user opts not to run the code, this result is returned.
/// - `Ok(_backslashResult_PSQL_CMD_ERROR)`: If any errors occur during processing, such as issues parsing the input
///   or generating the code, an error result is returned.
///
/// # Errors
/// - Returns an error if any part of the function fails, including processing the command input,
///   generating code, or handling the output buffers.
///
pub fn execute_command_generate(
    scan_state: PsqlScanState,
    query_buf: PQExpBuffer,
    pset: PsqlSettings,
) -> Result<backslashResult, Box<dyn Error>> {
    if scan_state.is_null() {
        return Ok(PSQL_CMD_ERROR);
    }

    let args = match extract_args(scan_state, OT_WHOLE_LINE, null_mut(), false) {
        Ok(args) => args,
        Err(_) => {
            return Ok(PSQL_CMD_ERROR);
        }
    };

    let mut arg_text = match args {
        Some(s) => s,
        None => match ask_additional_instructions("Jot instructions: ", false) {
            Ok(psqlx_utils::AdditionalInstructions::Text(text)) => text,
            Ok(psqlx_utils::AdditionalInstructions::Flag(true)) => {
                println!("No instructions given.");
                return Ok(PSQL_CMD_SKIP_LINE);
            }
            Ok(psqlx_utils::AdditionalInstructions::Flag(false)) => {
                println!("Meta-command cancelled.");
                return Ok(PSQL_CMD_SKIP_LINE);
            }
            Err(_) => return Ok(PSQL_CMD_ERROR),
        },
    };
    let mut spinner = Spinner::start();

    let schema = get_schema_json(pset);
    let mut additional_instruction: Option<String> = None;
    let mut messages: Vec<serde_json::Value> = Vec::new();

    loop {
        match generate_code(
            arg_text.as_str(),
            &mut messages,
            additional_instruction.clone(),
            &schema,
        ) {
            Ok(generated_code) => {
                spinner.stop();
                println!("{}", generated_code);

                match ask_additional_instructions("Follow up instructions, or", true) {
                    Ok(AdditionalInstructions::Text(text)) => {
                        // Continue the conversation with the new instruction
                        arg_text = text;
                        additional_instruction = None;
                        spinner = Spinner::start();
                    }
                    Ok(AdditionalInstructions::Flag(flag)) => match flag {
                        true => {
                            replace_query_buffer_data(query_buf, generated_code.as_str());
                            return Ok(PSQL_CMD_NEWEDIT);
                        }
                        false => {
                            println!("Meta-command cancelled.");
                            return Ok(PSQL_CMD_SKIP_LINE);
                        }
                    },
                    Err(_) => return Ok(PSQL_CMD_ERROR),
                }
            }
            Err(e) => {
                spinner.stop();
                println!("Error: {}", e);
                return Err(e);
            }
        }
    }
}

fn generate_code(
    arg_text: &str,
    messages: &mut Vec<serde_json::Value>,
    additional_instruction: Option<String>,
    schema: &str,
) -> Result<String, Box<dyn Error>> {
    // If this is the first message, initialize with system prompt
    let system_message= format!(
        "You are a Distinguished Engineer code assistant for PSQL, the Postgres terminal.
        Generate and return only the exact SQL it will be used as input to run again, do not use markdown and return the code formatted, nothing else.
        Current schema of the database is:
        {:?}
    ", schema);

    // Add the user's message to chat history
    messages.push(json!({
        "role": "user",
        "content": match additional_instruction {
            Some(instruction) => format!("Follow up instruction: {}", instruction),
            None => format!("Code generation request: {:?}", arg_text),
        }
    }));

    // Get the response
    let response = completion(messages, &system_message)?;

    // Add the assistant's response to the chat history
    messages.push(json!({
        "role": "assistant",
        "content": response.clone()
    }));

    Ok(response)
}
