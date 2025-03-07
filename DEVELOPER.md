## Creating Custom Libraries

To create your own custom meta-commands, use the `psqlx-utils` crate to access PSQL bindings.
If you need an example, check the `psqlx-ai` project.

### Example in Rust

```rust
use psqlx_utils::MetaCommand;

pub struct MyCommand;

impl MetaCommand for MyCommand {
    fn name(&self) -> &str {
        "custom_command"
    }

    fn execute(
        &self,
        scan_state: PsqlScanState,
        active_branch: bool,
        query_buf: PQExpBuffer,
        previous_buf: PQExpBuffer,
        pset: PsqlSettings,
    ) -> Result<_backslashResult, Box<dyn Error>> {
        println!("Hello world!");
    }
}
```

```rust
use psqlx_utils::Plugin;

struct AIPlugin;

impl Plugin for CustomPlugin {
    fn name(&self) -> &str {
        "custom_plugin_name"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn meta_commands(&self) -> Vec<Box<dyn MetaCommand>> {
        vec![Box::new(MyCommand)]
    }

    fn execute_command(
        &self,
        cmd: &str,
        scan_state: PsqlScanState,
        _active_branch: bool,
        query_buf: PQExpBuffer,
        previous_buf: PQExpBuffer,
        pset: PsqlSettings,
    ) -> _backslashResult {
        match cmd {
            "MyCommand" => match execute_command_generate(scan_state, query_buf, pset) {
                Ok(res) => res,
                Err(_) => _backslashResult_PSQL_CMD_ERROR,
            },
            _ => _backslashResult_PSQL_CMD_UNKNOWN,
        }
    }
}

```

Compile your library as a shared object (`.so` or `.dylib` on macOS) and place it in the appropriate directory (`/usr/local/pgsql/...`) for PSQLX to load.
