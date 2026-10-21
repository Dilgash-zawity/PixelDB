import type { ErrorKind } from './types';

export type InvokeFn = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

/** Error returned by a backend command, carrying the structured error kind. */
export class ApiError extends Error {
	constructor(
		public readonly kind: ErrorKind | 'ipc',
		message: string
	) {
		super(message);
		this.name = 'ApiError';
	}
}

let invokeImpl: InvokeFn | null = null;

/** Replaces the IPC transport (used by tests). */
export function setInvoke(fn: InvokeFn | null): void {
	invokeImpl = fn;
}

async function defaultInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
	const { invoke } = await import('@tauri-apps/api/core');
	return invoke<T>(cmd, args);
}

function toApiError(err: unknown): ApiError {
	if (err && typeof err === 'object' && 'kind' in err && 'message' in err) {
		const e = err as { kind: ErrorKind; message: string };
		return new ApiError(e.kind, e.message);
	}
	return new ApiError('ipc', err instanceof Error ? err.message : String(err));
}

/** Calls a backend command, converting failures into `ApiError`. */
export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
	try {
		return await (invokeImpl ?? defaultInvoke)<T>(cmd, args);
	} catch (err) {
		throw toApiError(err);
	}
}
