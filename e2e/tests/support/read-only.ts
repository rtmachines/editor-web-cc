import path from 'node:path';

// ADR-0113 (h): the second dev server follows this file. The read-only mode is on while it exists.

/** The absolute path of the switch file. The config and the read-only spec import it. */
export const READ_ONLY_FILE = path.resolve(__dirname, '..', '..', 'test-results', 'read-only-mode');
