#include "psqlx.h"

    /**
	 * Start of the PSQLX commands code.
	 */
     if (has_command_ext(cmd)) {
		status = exec_command_ext(cmd, scan_state, active_branch, query_buf, previous_buf, pset);
	}
	/**
	 * End of the PSQLX commands code.
	 */
	 else