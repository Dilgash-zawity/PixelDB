import { toast } from 'svelte-sonner';
import { ApiError } from '../api/client';
import { formatDetails, screenOf } from '../logic/diagnostics';
import { humanizeError } from '../logic/messages';
import { diagnosticsService, type Diagnostics } from '../services/diagnostics';

const ERROR_TITLES: Record<string, string> = {
	validation: 'Check the values',
	constraint: 'That would break a rule',
	not_found: 'Not found',
	already_exists: 'Already exists',
	invalid_identifier: 'Invalid name',
	invalid_path: 'Invalid location',
	invalid_database: 'Not a valid database',
	unsupported: 'Not supported',
	confirmation_required: 'Confirmation required',
	no_database: 'No database open',
	connection: "Couldn't connect"
};

/** Removes the generic "kind: " prefix the backend adds to messages. */
export function cleanMessage(message: string): string {
	return message.replace(
		/^(validation failed|constraint violation|not found|already exists|invalid identifier|unsupported operation|database error|invalid path|confirmation required|could not connect): /i,
		''
	);
}

let facts: Promise<Diagnostics | null> | null = null;
/** App version and system, read once. */
export function diagnostics(): Promise<Diagnostics | null> {
	facts ??= diagnosticsService.get().catch(() => null);
	return facts;
}

/** Copies a bug-report-ready description of an error. */
async function copyDetails(title: string, message: string, kind?: string) {
	const d = await diagnostics();
	const text = formatDetails({
		title,
		message,
		kind,
		screen: screenOf(location.pathname),
		version: d?.version,
		os: d ? `${d.os} ${d.arch}, SQLite ${d.sqlite}` : navigator.userAgent
	});
	try {
		await navigator.clipboard.writeText(text);
		toast.success('Copied the details', { description: 'Paste them into your bug report.' });
	} catch {
		toast.error('Couldn’t copy', { description: text });
	}
}

/** Kinds that mean something went wrong in the app rather than with what the user typed; they go to the log file. */
const UNEXPECTED = ['ipc', 'sqlite', 'io', 'internal', 'invalid_database'];

export function reportError(e: unknown) {
	const kind = e instanceof ApiError ? e.kind : undefined;
	const title = (kind && ERROR_TITLES[kind]) ?? 'Something went wrong';
	const message = e instanceof ApiError ? humanizeError(cleanMessage(e.message)) : e instanceof Error ? e.message : String(e);
	if (!kind || UNEXPECTED.includes(kind)) logError(kind ?? 'error', e);
	toast.error(title, {
		description: message,
		action: { label: 'Copy details', onClick: () => void copyDetails(title, message, kind) }
	});
}

/** Writes an unexpected error to the app's log file (never sent anywhere). */
export function logError(context: string, e: unknown) {
	const message = e instanceof Error ? `${e.message}${e.stack ? `\n${e.stack}` : ''}` : String(e);
	diagnosticsService.logError(`${screenOf(location.pathname)} ${context}`, message).catch(() => {});
}

/** Runs an action and reports failures as a toast; returns undefined on error. */
export async function attempt<T>(action: () => Promise<T>): Promise<T | undefined> {
	try {
		return await action();
	} catch (e) {
		reportError(e);
		return undefined;
	}
}

export interface ConfirmOptions {
	title: string;
	description: string;
	action?: string;
	destructive?: boolean;
	/**
	 * Which ConfirmHost shows the dialog. Overlays such as sheets render their
	 * own scoped host inside their content so the dialog stacks above them.
	 */
	scope?: string;
}

/** State behind the single app-wide confirmation dialog. */
class Confirmation {
	open = $state(false);
	options = $state<ConfirmOptions>({ title: '', description: '' });
	private resolve: ((ok: boolean) => void) | null = null;

	ask(options: ConfirmOptions): Promise<boolean> {
		this.resolve?.(false);
		this.options = options;
		this.open = true;
		return new Promise((resolve) => (this.resolve = resolve));
	}

	settle(ok: boolean) {
		this.open = false;
		this.resolve?.(ok);
		this.resolve = null;
	}
}

export const confirmation = new Confirmation();
export const confirm = (options: ConfirmOptions) => confirmation.ask(options);
