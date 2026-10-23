import type { DownloadEvent } from '@tauri-apps/plugin-updater';

export interface AvailableUpdate {
	version: string;
	body?: string;
	download(onEvent: (event: DownloadEvent) => void, options: { timeout: number }): Promise<void>;
	install(): Promise<void>;
	close(): Promise<void>;
}

export interface UpdateSnapshot {
	status: 'idle' | 'checking' | 'current' | 'available' | 'confirming' | 'downloading' | 'installing' | 'ready' | 'error';
	version: string | null;
	notes: string;
	message: string;
	progress: number | null;
}

export const initialUpdateSnapshot: UpdateSnapshot = {
	status: 'idle', version: null, notes: '', message: '', progress: null
};

/** Keeps checks and installs sequential, and never installs an unverified download. */
export class UpdateManager {
	private update: AvailableUpdate | null = null;
	private snapshot: UpdateSnapshot = { ...initialUpdateSnapshot };
	private busy = false;

	constructor(
		private readonly fetchUpdate: () => Promise<AvailableUpdate | null>,
		private readonly restartApp: () => Promise<void>,
		private readonly changed: (snapshot: UpdateSnapshot) => void
	) {}

	private set(patch: Partial<UpdateSnapshot>) {
		this.snapshot = { ...this.snapshot, ...patch };
		this.changed(this.snapshot);
	}

	async check() {
		if (this.busy || this.snapshot.status === 'ready') return;
		this.busy = true;
		this.set({ status: 'checking', message: '', progress: null });
		try {
			const next = await this.fetchUpdate();
			await this.update?.close().catch(() => {});
			this.update = next;
			this.set({
				status: next ? 'available' : 'current',
				version: next?.version ?? null,
				notes: next?.body ?? '',
				message: next ? `SlateDB ${next.version} is available.` : 'You’re up to date.'
			});
		} catch {
			this.set({ status: 'error', message: 'Couldn’t check for updates. Check your connection and try again. You can keep using SlateDB offline.' });
		} finally {
			this.busy = false;
		}
	}

	async install(approve: () => Promise<boolean>) {
		if (this.busy || !this.update || this.snapshot.status === 'ready') return;
		this.busy = true;
		this.set({ status: 'confirming', message: '' });
		try {
			if (!(await approve())) {
				this.set({ status: 'available' });
				return;
			}
			this.set({ status: 'downloading', progress: null });
			let downloaded = 0;
			let total: number | undefined;
			await this.update.download((event) => {
				if (event.event === 'Started') total = event.data.contentLength;
				if (event.event === 'Progress') downloaded += event.data.chunkLength;
				this.set({ progress: total ? Math.min(100, Math.round(downloaded / total * 100)) : null });
			}, { timeout: 300_000 });
			// Tauri verifies the signature during download; a failure never reaches install.
			this.set({ status: 'installing', message: 'Installing update…' });
			await this.update.install(); // Windows exits here and its installer restarts the app.
			this.set({ status: 'ready', message: 'Update installed. Restart SlateDB to finish.' });
			await this.update.close().catch(() => {});
			this.update = null;
		} catch {
			this.set({ status: 'error', message: 'The update couldn’t be verified or installed. Try again, or download the latest installer from the official releases page.' });
		} finally {
			this.busy = false;
		}
	}

	async restart() {
		if (this.busy || this.snapshot.status !== 'ready') return;
		this.busy = true;
		try {
			await this.restartApp();
		} catch {
			this.set({ message: 'Please close and reopen SlateDB to finish updating.' });
		} finally {
			this.busy = false;
		}
	}
}
