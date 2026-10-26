import { call } from '../api/client';

export interface Diagnostics {
	version: string;
	os: string;
	arch: string;
	sqlite: string;
}

/** App and system facts for bug reports, and the local error log. */
export const diagnosticsService = {
	get: () => call<Diagnostics>('diagnostics'),
	logError: (context: string, message: string) => call<void>('log_client_error', { context, message }),
	openLogFolder: () => call<void>('open_log_folder')
};
