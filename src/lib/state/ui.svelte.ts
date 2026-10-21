import { ApiError } from '../api/client';

export interface ConfirmOptions {
	title: string;
	message: string;
	confirmLabel?: string;
	danger?: boolean;
}

export interface PromptOptions {
	title: string;
	label: string;
	value?: string;
	confirmLabel?: string;
	/** Returns an error message, or null when the value is acceptable. */
	validate?: (value: string) => string | null;
}

type DialogRequest =
	| ({ kind: 'confirm'; resolve: (ok: boolean) => void } & ConfirmOptions)
	| ({ kind: 'prompt'; resolve: (value: string | null) => void } & PromptOptions);

export interface MenuItem {
	label?: string;
	icon?: string;
	danger?: boolean;
	disabled?: boolean;
	separator?: boolean;
	action?: () => void;
}

export interface Toast {
	id: number;
	kind: 'info' | 'success' | 'error';
	title: string;
	message: string;
}

const ERROR_TITLES: Record<string, string> = {
	validation: 'Invalid value',
	constraint: 'Data rule violated',
	not_found: 'Not found',
	already_exists: 'Already exists',
	invalid_identifier: 'Invalid name',
	invalid_path: 'Invalid location',
	invalid_database: 'Not a valid database',
	unsupported: 'Not supported',
	confirmation_required: 'Confirmation required',
	no_database: 'No database open'
};

/** Strips the generic "kind: " prefix the backend puts on messages. */
function cleanMessage(message: string): string {
	return message.replace(/^(validation failed|constraint violation|not found|already exists|invalid identifier|unsupported operation|database error|invalid path): /i, '');
}

/** App-wide dialogs, context menus and notifications. */
class Ui {
	dialog = $state<DialogRequest | null>(null);
	menu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);
	toasts = $state<Toast[]>([]);
	private nextToast = 1;

	confirm(opts: ConfirmOptions): Promise<boolean> {
		this.cancelDialog();
		return new Promise((resolve) => (this.dialog = { kind: 'confirm', resolve, ...opts }));
	}

	prompt(opts: PromptOptions): Promise<string | null> {
		this.cancelDialog();
		return new Promise((resolve) => (this.dialog = { kind: 'prompt', resolve, ...opts }));
	}

	private cancelDialog() {
		const d = this.dialog;
		if (!d) return;
		this.dialog = null;
		if (d.kind === 'confirm') d.resolve(false);
		else d.resolve(null);
	}

	closeDialog(result: boolean | string | null) {
		const d = this.dialog;
		if (!d) return;
		this.dialog = null;
		if (d.kind === 'confirm') d.resolve(result === true);
		else d.resolve(typeof result === 'string' ? result : null);
	}

	openMenu(event: MouseEvent, items: MenuItem[]) {
		event.preventDefault();
		event.stopPropagation();
		this.menu = { x: event.clientX, y: event.clientY, items };
	}

	notify(kind: Toast['kind'], title: string, message = '') {
		const toast = { id: this.nextToast++, kind, title, message };
		this.toasts.push(toast);
		setTimeout(() => this.dismiss(toast.id), kind === 'error' ? 8000 : 3500);
	}

	error(e: unknown) {
		if (e instanceof ApiError) {
			this.notify('error', ERROR_TITLES[e.kind] ?? 'Error', cleanMessage(e.message));
		} else {
			this.notify('error', 'Error', e instanceof Error ? e.message : String(e));
		}
	}

	dismiss(id: number) {
		this.toasts = this.toasts.filter((t) => t.id !== id);
	}

	/** Runs an action and reports failures; returns undefined on error. */
	async attempt<T>(action: () => Promise<T>): Promise<T | undefined> {
		try {
			return await action();
		} catch (e) {
			this.error(e);
			return undefined;
		}
	}
}

export const ui = new Ui();
