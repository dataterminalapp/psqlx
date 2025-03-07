# PSQLX - A PSQL Fork Focused on AI and Extensibility

PSQLX is an open-source project that extends PSQL by enabling custom meta-commands written in Rust.

## Features

- **Extensibility:** Load plugins to introduce new meta-commands.
- **Rust Support:** Write meta-commands in Rust.
- **Seamless Integration:** Works as a drop-in replacement for PSQL.
- **Powered by `psqlx-utils` and `psqlx-sys`:** Provides the necessary PSQL bindings and utility functions to create custom meta-commands.

## How Does PSQLX Work?

PSQLX is a fork of PSQL with a subtle modification that enables dynamic library loading. The core PSQL functionality remains unchanged, while additional meta-commands are handled by external libraries.

## Installation

### macOS (Apple Silicon)

1. Install `psqlx` using Homebrew:

   ```sh
   brew install dataterminalapp/tap/psqlx
   ```

1. Enable AI-powered commands:

   ```sh
   # OpenAI
   export OPENAI_API_KEY=<API_KEY>

   # Or Anthropic/Claude
   export PSQLX_AI_PROVIDER=anthropic
   export ANTHROPIC_API_KEY=<API_KEY>
   ```

### Linux

WIP

### Building from Source

For building from source refer to `BUILD.md` for more instruction.

### Running PSQLX

Once installed, you can use PSQLX as a drop-in replacement for PSQL:

```sh
psqlx -U myuser -d mydatabase
```

## Example Usage

`psqlx` provides built-in AI-powered meta-commands to assist with SQL queries.

### Fixing Errors with `\fix`

If you encounter a SQL error, the `\fix` meta-command suggests a corrected query:

```sh
postgres=# SELECT * FROM pg_columns;
ERROR:  relation "pg_columns" does not exist
LINE 1: SELECT * FROM pg_columns;
                      ^
postgres=# \fix
SELECT * FROM information_schema.columns;
Run fix? [enter/esc]:
```

### Generating Queries with `\generate`

The `\generate` meta-command helps create SQL queries based on natural language input:

```sh
postgres=# \generate a random query

SELECT user_id, COUNT(*) AS click_count
FROM clicks
WHERE converted = TRUE
GROUP BY user_id
ORDER BY click_count DESC
LIMIT 10;

Follow up instructions, or [enter/esc]:

 user_id | click_count
---------+-------------
      68 |          11
     283 |           9
      73 |           8
     994 |           8
     739 |           8
     423 |           8
     390 |           8
     978 |           8
     447 |           8
     599 |           8
(10 rows)

```

## Creating Your Own Meta-Command

Building a meta-command is really simple! Follow the instructions in `DEVELOPER.md`.

## Contributing

We welcome contributions! Feel free to submit pull requests, report issues, or suggest features.

---

### Future Plans

- Support for linux and apt.
- Plugin discovery & management.
