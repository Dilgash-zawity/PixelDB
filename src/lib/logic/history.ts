/**
 * Undo/redo of a document kept as JSON snapshots. Call `record` with the
 * state *before* each change; consecutive changes with the same `group`
 * (e.g. typing into one text box) are merged into one step.
 */
export class History<T> {
	private past: { snapshot: string; group?: string }[] = [];
	private future: string[] = [];

	constructor(private readonly limit = 100) {}

	record(before: T, group?: string): void {
		const last = this.past.at(-1);
		this.future = [];
		if (group && last?.group === group) return;
		this.past.push({ snapshot: JSON.stringify(before), group });
		if (this.past.length > this.limit) this.past.shift();
	}

	/** Ends merging, so the next change is a step of its own. */
	seal(): void {
		const last = this.past.at(-1);
		if (last) last.group = undefined;
	}

	get canUndo(): boolean {
		return this.past.length > 0;
	}

	get canRedo(): boolean {
		return this.future.length > 0;
	}

	/** The state to go back to, given the current one. */
	undo(current: T): T | undefined {
		const step = this.past.pop();
		if (!step) return undefined;
		this.future.push(JSON.stringify(current));
		return JSON.parse(step.snapshot);
	}

	redo(current: T): T | undefined {
		const next = this.future.pop();
		if (next === undefined) return undefined;
		this.past.push({ snapshot: JSON.stringify(current) });
		return JSON.parse(next);
	}

	clear(): void {
		this.past = [];
		this.future = [];
	}
}
