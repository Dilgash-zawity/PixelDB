import { CARD_WIDTH, roundedPath, type Point } from './relationships';

/** One line on the home screen's map: an app using a database. */
export interface WorkspaceLink {
	app: string;
	database: string;
	/** The app names a database that isn't in the folder. */
	missing: boolean;
}

export type NodeKind = 'app' | 'db';

/** Card positions on the home canvas, keyed by `nodeKey`. */
export type MapLayout = Record<string, Point>;

/** Cards are as wide as diagram tables so lines route the same way. */
export const NODE_WIDTH = CARD_WIDTH;
export const NODE_HEIGHT = 92;
const COLUMN_GAP = 200;
const ROW_GAP = 28;

/** Where the home map keeps card positions for a folder (browser storage). */
export function homeLayoutKey(folder: string): string {
	return `slatedb.home-layout:${folder}`;
}

export function nodeKey(kind: NodeKind, name: string): string {
	return `${kind}:${name}`;
}

/** Every app → database connection, in app order, skipping unreadable apps. */
export function workspaceLinks(
	apps: { name: string; databases: string[]; valid: boolean }[],
	databases: string[]
): WorkspaceLink[] {
	const present = new Set(databases);
	return apps.flatMap((app) =>
		app.valid
			? [...new Set(app.databases)].map((database) => ({ app: app.name, database, missing: !present.has(database) }))
			: []
	);
}

/** Apps using each database, for "Used by" counts. */
export function usersByDatabase(links: WorkspaceLink[]): Map<string, string[]> {
	const users = new Map<string, string[]>();
	for (const l of links) users.set(l.database, [...(users.get(l.database) ?? []), l.app]);
	return users;
}

/**
 * Apps in a column on the left, databases on the right, each database level
 * with the average of the apps using it so lines cross as little as possible.
 */
export function autoMapLayout(apps: string[], databases: string[], links: WorkspaceLink[]): MapLayout {
	const layout: MapLayout = {};
	const step = NODE_HEIGHT + ROW_GAP;
	const appRow = new Map(apps.map((a, i) => [a, i]));
	const weight = (db: string) => {
		const rows = links.filter((l) => l.database === db && appRow.has(l.app)).map((l) => appRow.get(l.app)!);
		return rows.length ? rows.reduce((a, b) => a + b, 0) / rows.length : Infinity;
	};
	const order = databases.map((d, i) => ({ d, i, w: weight(d) })).sort((a, b) => a.w - b.w || a.i - b.i);
	const offset = (count: number, other: number) => (Math.max(0, other - count) * step) / 2;
	apps.forEach((a, i) => (layout[nodeKey('app', a)] = { x: 0, y: offset(apps.length, databases.length) + i * step }));
	order.forEach(({ d }, i) => {
		layout[nodeKey('db', d)] = {
			x: apps.length ? NODE_WIDTH + COLUMN_GAP : 0,
			y: offset(databases.length, apps.length) + i * step
		};
	});
	return layout;
}

function overlaps(layout: MapLayout, p: Point, except?: string): boolean {
	return Object.entries(layout).some(
		([k, q]) =>
			k !== except && Math.abs(q.x - p.x) < NODE_WIDTH + 16 && Math.abs(q.y - p.y) < NODE_HEIGHT + 12
	);
}

/** Moves `p` down until it doesn't sit on another card. */
export function freeSpot(layout: MapLayout, p: Point, except?: string): Point {
	const at = { ...p };
	while (overlaps(layout, at, except)) at.y += NODE_HEIGHT + ROW_GAP;
	return at;
}

/**
 * Keeps saved positions, drops cards that no longer exist, and places new
 * ones: a database next to an app that uses it, otherwise below its kind.
 */
export function placeMissingNodes(saved: MapLayout, apps: string[], databases: string[], links: WorkspaceLink[]): MapLayout {
	const keys = new Set([...apps.map((a) => nodeKey('app', a)), ...databases.map((d) => nodeKey('db', d))]);
	const layout: MapLayout = Object.fromEntries(Object.entries(saved).filter(([k]) => keys.has(k)));
	if (!Object.keys(layout).length) return autoMapLayout(apps, databases, links);
	const placed = (kind: NodeKind) => Object.entries(layout).filter(([k]) => k.startsWith(`${kind}:`)).map(([, p]) => p);
	const below = (kind: NodeKind, fallbackX: number): Point => {
		const ps = placed(kind);
		if (!ps.length) return { x: fallbackX, y: Math.min(...Object.values(layout).map((p) => p.y)) };
		return { x: Math.min(...ps.map((p) => p.x)), y: Math.max(...ps.map((p) => p.y)) + NODE_HEIGHT + ROW_GAP };
	};
	const allX = () => Object.values(layout).map((p) => p.x);
	for (const a of apps) {
		const k = nodeKey('app', a);
		if (!layout[k]) layout[k] = freeSpot(layout, below('app', Math.min(...allX()) - NODE_WIDTH - COLUMN_GAP));
	}
	for (const d of databases) {
		const k = nodeKey('db', d);
		if (layout[k]) continue;
		const user = links.find((l) => l.database === d && layout[nodeKey('app', l.app)]);
		const app = user && layout[nodeKey('app', user.app)];
		layout[k] = freeSpot(
			layout,
			app ? { x: app.x + NODE_WIDTH + COLUMN_GAP, y: app.y } : below('db', Math.max(...allX()) + NODE_WIDTH + COLUMN_GAP)
		);
	}
	return layout;
}

/** A short dashed stub out of an app's right edge for a database that's gone. */
export function missingStub(app: Point): { path: string; end: Point } {
	const y = app.y + NODE_HEIGHT / 2;
	const end = { x: app.x + NODE_WIDTH + 64, y };
	return { path: roundedPath([{ x: app.x + NODE_WIDTH, y }, end]), end };
}
