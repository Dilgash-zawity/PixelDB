import { toast } from 'svelte-sonner';
import { ApiError } from '../api/client';
import { humanizeError } from '../logic/messages';

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

export function reportError(e: unknown) {
	if (e instanceof ApiError) {
		toast.error(ERROR_TITLES[e.kind] ?? 'Something went wrong', { description: humanizeError(cleanMessage(e.message)) });
	} else {
		toast.error('Something went wrong', { description: e instanceof Error ? e.message : String(e) });
	}
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
