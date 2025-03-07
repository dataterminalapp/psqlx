pub mod spinner;
use std::{
    error::Error,
    ffi::{CStr, CString},
    io::{self, Write},
};

use psqlx_sys::{
    appendPQExpBufferStr, backslashResult,
    backslashResult::PSQL_CMD_ERROR,
    psql_scan_slash_option, resetPQExpBuffer, slash_option_type,
    ExecStatusType::{PGRES_COMMAND_OK, PGRES_TUPLES_OK},
    PGresult, PQExpBuffer, PQExpBufferData, PQclear, PQerrorMessage, PQexec, PQgetisnull,
    PQgetvalue, PQnfields, PQntuples, PQresultErrorMessage, PQresultStatus, PsqlScanState,
    PsqlSettings,
};

// Trait that all PSQL meta-commands must implement
pub trait MetaCommand: Send + Sync {
    fn name(&self) -> &str;
    fn execute(
        &self,
        scan_state: PsqlScanState,
        active_branch: bool,
        query_buf: PQExpBuffer,
        previous_buf: PQExpBuffer,
        pset: PsqlSettings,
    ) -> Result<backslashResult, Box<dyn std::error::Error>>;
}

// Trait that all plugins must implement
pub trait Plugin {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn meta_commands(&self) -> Vec<Box<dyn MetaCommand>>;
    fn execute_command(
        &self,
        cmd: &str,
        scan_state: PsqlScanState,
        active_branch: bool,
        query_buf: PQExpBuffer,
        previous_buf: PQExpBuffer,
        pset: PsqlSettings,
    ) -> backslashResult {
        self.meta_commands()
            .iter()
            .find(|meta_cmd| meta_cmd.name() == cmd)
            .unwrap()
            .execute(scan_state, active_branch, query_buf, previous_buf, pset)
            .unwrap_or(PSQL_CMD_ERROR)
    }
}

pub fn to_c_str(string: &str) -> *const i8 {
    return match CString::new(string) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => std::ptr::null_mut(),
    };
}

pub fn to_rust_string(ptr: *const i8) -> Result<String, Box<dyn Error>> {
    if ptr.is_null() {
        return Err("Null pointer".into());
    }
    let c_str = unsafe { CStr::from_ptr(ptr) };
    Ok(c_str.to_str()?.to_string())
}

pub enum AdditionalInstructions {
    Text(String),
    Flag(bool),
}

use crossterm::{
    cursor,
    event::{read, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};

pub fn ask_yea_or_nay(question: &str) -> Result<bool, Box<dyn Error>> {
    // Enable raw mode to capture individual keystrokes
    enable_raw_mode()?;

    // Print the initial prompt
    let mut stdout = io::stdout();
    execute!(stdout, cursor::SavePosition)?;
    print!("{} [enter/esc]: ", question);
    stdout.flush()?;

    // Initialize an empty string to store user input
    let input = String::new();
    loop {
        match read()? {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => match (code, modifiers) {
                // Enter key - return the input
                (KeyCode::Enter, _) => {
                    disable_raw_mode()?;

                    // If input is empty, return a Flag(true)
                    let trimmed_input = input.trim().to_string();
                    if trimmed_input.is_empty() {
                        return Ok(true);
                    } else {
                        return Ok(false);
                    }
                }

                // Escape - cancel operation
                (KeyCode::Esc, _) => {
                    disable_raw_mode()?;
                    return Ok(false);
                }

                // Ctrl+C or Cmd+C handling
                (KeyCode::Char('c'), KeyModifiers::CONTROL)
                | (KeyCode::Char('c'), KeyModifiers::META) => {
                    disable_raw_mode()?;
                    return Ok(false);
                }

                _ => {} // Ignore other keys
            },
            _ => {}
        }
    }
}

pub fn ask_additional_instructions(
    text: &str,
    help: bool,
) -> Result<AdditionalInstructions, Box<dyn Error>> {
    // Enable raw mode to capture individual keystrokes
    enable_raw_mode()?;

    // Print the initial prompt
    let mut stdout = io::stdout();
    execute!(stdout, cursor::SavePosition)?;
    if help {
        print!("{} [enter/esc]: ", text);
    } else {
        print!("{}", text);
    }

    stdout.flush()?;

    // Initialize an empty string to store user input
    let mut input = String::new();

    loop {
        match read()? {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => match (code, modifiers) {
                // Enter key - return the input
                (KeyCode::Enter, _) => {
                    disable_raw_mode()?;
                    println!(); // Move to next line after enter

                    // If input is empty, return a Flag(true)
                    let trimmed_input = input.trim().to_string();
                    if trimmed_input.is_empty() {
                        return Ok(AdditionalInstructions::Flag(true));
                    }

                    let lowercase_trimmed_input = trimmed_input.to_lowercase();
                    if lowercase_trimmed_input == "quit"
                        || lowercase_trimmed_input == "n"
                        || lowercase_trimmed_input == "\\q"
                        || lowercase_trimmed_input == "exit"
                        || lowercase_trimmed_input == "\\exit"
                    {
                        return Ok(AdditionalInstructions::Flag(false));
                    }

                    // Return input as Text
                    return Ok(AdditionalInstructions::Text(input.trim().to_string()));
                }

                // Backspace - remove last character
                (KeyCode::Backspace, _) => {
                    if !input.is_empty() {
                        input.pop();
                        execute!(stdout, cursor::MoveLeft(1), cursor::SavePosition)?;
                        print!(" ");
                        execute!(stdout, cursor::RestorePosition)?;
                        stdout.flush()?;
                    }
                }

                // Ctrl+C or Cmd+C handling
                (KeyCode::Char('c'), KeyModifiers::CONTROL)
                | (KeyCode::Char('c'), KeyModifiers::META) => {
                    disable_raw_mode()?;
                    return Ok(AdditionalInstructions::Flag(false));
                }

                // Character input
                (KeyCode::Char(c), _) => {
                    input.push(c);
                    print!("{}", c);
                    stdout.flush()?;
                }

                // Escape - cancel operation
                (KeyCode::Esc, _) => {
                    disable_raw_mode()?;
                    return Ok(AdditionalInstructions::Flag(false));
                }

                _ => {} // Ignore other keys
            },

            // Handle other events if needed
            _ => {}
        }
    }
}

pub fn replace_query_buffer_data(query_buf: *mut PQExpBufferData, str: &str) {
    unsafe {
        resetPQExpBuffer(query_buf);
        appendPQExpBufferStr(query_buf, to_c_str(str));
    }
}

pub fn extract_args(
    state: PsqlScanState,
    type_: slash_option_type,
    quote: *mut ::std::os::raw::c_char,
    semicolon: bool,
) -> Result<Option<String>, Box<dyn Error>> {
    let curline = unsafe { to_rust_string((*state).curline)? };
    let refline = unsafe { to_rust_string((*state).refline)? };

    if curline.trim() == refline.trim() {
        return Ok(None);
    }

    let arg_text = unsafe { psql_scan_slash_option(state, type_, quote, semicolon) };

    Ok(Some(to_rust_string(arg_text)?))
}

/// Converts a PQExpBuffer to a Rust String
///
/// # Arguments
/// * `buffer` - A pointer to a PQExpBufferData structure
///
/// # Returns
/// * `Result<String, Box<dyn Error>>` - The string content of the buffer or an error
///
/// # Safety
/// This function handles raw pointers and assumes the PQExpBuffer is valid
pub fn pqexpbuffer_to_string(buffer: PQExpBuffer) -> Result<String, Box<dyn Error>> {
    // Check if buffer is null
    if buffer.is_null() {
        return Err("Null PQExpBuffer pointer".into());
    }

    unsafe {
        // Get reference to buffer data
        let buf = &*buffer;

        // Check if data pointer is null or length is 0
        if buf.data.is_null() || buf.len == 0 {
            return Ok(String::new());
        }

        // Create a slice from the raw char pointer
        let slice = std::slice::from_raw_parts(buf.data as *const u8, buf.len);

        // Try to convert the byte slice to a String
        match std::str::from_utf8(slice) {
            Ok(s) => Ok(s.to_string()),
            Err(e) => Err(format!("Invalid UTF-8 sequence in PQExpBuffer: {}", e).into()),
        }
    }
}

/// SQL query to retrieve tables, views, and their columns in the current session context.
pub const SCHEMA_QUERY: &str = r#"
SELECT json_agg(row_to_json(schema_info))
FROM (
  SELECT t.table_name, t.table_type, t.table_schema,
         c.column_name, c.data_type
  FROM information_schema.tables t
  JOIN information_schema.columns c ON t.table_name = c.table_name
  WHERE t.table_schema NOT IN ('pg_catalog', 'information_schema')
  ORDER BY t.table_schema, t.table_name, c.ordinal_position
) AS schema_info;
"#;

/// Retrieves the schema information (tables, views, and columns) for the current session context in a JSON string (`SELECT json_agg(row_to_json(schema_info))`).
///
/// This function executes a predefined SQL query (`SCHEMA_QUERY`) against the PostgreSQL database
/// to fetch details about tables, views, and columns. The query returns the schema information
/// in JSON format, which is then extracted and returned as a string.
///
/// # Arguments
/// - `pset`: The current PostgreSQL session settings (`PsqlSettings`), which contains the database connection.
///
/// # Returns
/// - A `String` containing the JSON-encoded schema information if the query is successful.
/// - An empty string if the query fails or no schema information is available.
///
/// # Example
/// ```rust
/// let schema_info = get_schema_json(pset);
/// println!("Schema: {}", schema_info);
/// ```
pub fn get_schema_json(pset: PsqlSettings) -> String {
    // Use our generic run_sql function for the schema query
    match query_as(pset, SCHEMA_QUERY, |values| {
        // For this case, we're expecting a single JSON column in a single row
        if values.is_empty() {
            return Ok("".to_string());
        }

        // Extract the JSON value, defaulting to empty string if NULL
        let json_str = values[0].unwrap_or("").to_string();
        Ok(json_str)
    }) {
        Ok(results) => {
            // Take the first row if available, otherwise return empty string
            results.into_iter().next().unwrap_or_else(|| "".to_string())
        }
        Err(_) => {
            // Return empty string on error, matching original behavior
            "".to_string()
        }
    }
}

pub fn run_sql<T, F>(pset: PsqlSettings, sql: &str, row_mapper: F) -> Result<Vec<T>, String>
where
    F: Fn(usize, usize, &[Option<&str>]) -> Result<T, String>,
{
    // Execute the query using the provided SQL string
    let req_res = unsafe { PQexec(pset.db, to_c_str(sql)) };

    // Check if the result pointer is null (connection error)
    if req_res.is_null() {
        let error_msg = unsafe { CStr::from_ptr(PQerrorMessage(pset.db)) }
            .to_string_lossy()
            .to_string();
        return Err(format!("Query execution failed: {}", error_msg));
    }

    // Ensure we clean up the result object to avoid memory leaks
    struct ResultGuard(*mut PGresult);
    impl Drop for ResultGuard {
        fn drop(&mut self) {
            unsafe { PQclear(self.0) };
        }
    }
    let _guard = ResultGuard(req_res);

    // Get the result status
    let status = unsafe { PQresultStatus(req_res) };

    // Process the result based on status
    match status {
        // Handle case when query returns data rows
        PGRES_TUPLES_OK => {
            let num_rows = unsafe { PQntuples(req_res) };
            let num_cols = unsafe { PQnfields(req_res) };

            let mut results = Vec::with_capacity(num_rows as usize);

            // Iterate over all rows
            for row_idx in 0..num_rows {
                // Collect all column values for this row
                let mut row_values = Vec::with_capacity(num_cols as usize);

                for col_idx in 0..num_cols {
                    // Check if the field is NULL
                    let is_null = unsafe { PQgetisnull(req_res, row_idx, col_idx) } != 0;

                    if is_null {
                        row_values.push(None);
                    } else {
                        // Get the field value
                        let value = unsafe { PQgetvalue(req_res, row_idx, col_idx) };
                        let value_str = unsafe { CStr::from_ptr(value) }.to_str().unwrap_or("");
                        row_values.push(Some(value_str));
                    }
                }

                // Let the caller map the row to their desired type
                match row_mapper(row_idx.try_into().unwrap(), num_cols as usize, &row_values) {
                    Ok(mapped_row) => results.push(mapped_row),
                    Err(err) => return Err(format!("Error mapping row {}: {}", row_idx, err)),
                }
            }

            Ok(results)
        }

        // Handle case when query executed successfully but doesn't return rows
        PGRES_COMMAND_OK => {
            // For operations like INSERT, UPDATE, DELETE
            Err("Command executed successfully but returned no data rows.".to_string())
        }

        // Handle error cases
        _ => {
            let error_msg = unsafe { CStr::from_ptr(PQresultErrorMessage(req_res)) }
                .to_string_lossy()
                .to_string();
            Err(format!("Query failed: {}", error_msg))
        }
    }
}

// Example implementation: A utility function that builds on run_sql for common query patterns
pub fn query_as<T, F>(pset: PsqlSettings, sql: &str, row_mapper: F) -> Result<Vec<T>, String>
where
    F: Fn(&[Option<&str>]) -> Result<T, String>,
{
    run_sql(pset, sql, |_, _, values| row_mapper(values))
}
