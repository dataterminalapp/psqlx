1. Clone the repository:
   ```sh
   git clone https://github.com/dataterminalapp/psqlx.git
   cd psqlx
   ```
2. Build and install the project:
   ```sh
   ./bin/build.sh
   ```

# Load libraries

```sh
# Load library (macOS)
cp target/release/libpsqlx_ai.dylib ~/.local/share/psqlx/plugins/


# Load library (Linux)
cp target/release/libpsqlx_ai.so ~/.local/share/psqlx/plugins/
```

# Use the build

```
/usr/local/pgsql/bin/psqlx "postgresql://postgres:postgres@localhost:5432"
```

# Bindings Generation

```bash
cargo install bindgen-cli

bindgen external/psql/src/include/wrapper.h  --output bindings.rs -- -I external/psql/src/include -I external/psql/src/interfaces/libpq -I external/psql/src/include/utils/ -I external/psql/src/include/utils/fe-utils
```
