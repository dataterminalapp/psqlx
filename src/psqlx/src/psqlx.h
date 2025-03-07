#ifndef PSQLX_H
#define PSQLX_H

#include "settings.h"

extern int has_command_ext(const char* cmd);
extern enum _backslashResult exec_command_ext(
    const char *cmd, 
    PsqlScanState scan_state, 
    bool active_branch, 
    PQExpBuffer query_buf, 
    PQExpBuffer previous_buf,
    PsqlSettings pset
);


#endif // PSQLX_H
