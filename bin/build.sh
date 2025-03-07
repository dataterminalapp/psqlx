#!/bin/bash
set -e

if [ ! -d "external/psql/.git" ]; then
    git submodule add -f https://github.com/postgres/postgres.git external/psql
    cd external/psql
    git checkout f4694e0
else 
    cd external/psql
fi

# Configure PSQL
./configure --with-openssl

make clean
make -C src/interfaces/libpq
make install -C src/interfaces/libpq
make -C src/common
make -C src/fe_utils/
make generated-headers -C src/backend
make -C src/bin/psql

cd ../..

# Add paths to make bindings work
REQUIRED_PATHS=(
"$(pwd)/external/psql/src/interfaces/libpq"
"$(pwd)/external/psql/src/include"
"$(pwd)/external/psql/src/include/utils"
"$(pwd)/external/psql/src/include/fe_utils"
"$(pwd)/external/psql/src/bin/psql"
)
MISSING_PATHS=()

for path in "${REQUIRED_PATHS[@]}"; do
    if [[ ":$CPATH:" != *":$path:"* ]]; then
        MISSING_PATHS+=("$path")
    fi
done

if [ ${#MISSING_PATHS[@]} -ne 0 ]; then
    # Join paths with ':' using printf to avoid extra colons
    export CPATH="$(printf '%s:' "${MISSING_PATHS[@]}")$CPATH"
    echo "Updated CPATH: $CPATH"
else
    echo "All required paths are already in CPATH."
fi

# Apply updates into PSQL
chmod +x ./src/psqlx/bin/updater.sh
src/psqlx/bin/updater.sh

cargo build --release

# If having permissions issue here use give yourself permissions over that dir only: sudo chown -R $(whoami):$(id -gn) /usr/local/pgsql
make -C external/psql/src/bin/psql
make install -C external/psql/src/bin/psql