/** Look-and-feel preferences for this computer, kept in the webview's local storage. */

export const ACCENTS = ['neutral', 'blue', 'green', 'amber', 'red', 'violet'] as const;
export const RADII = ['auto', 'none', 'small', 'medium', 'large'] as const;
export const TEXT_SIZES = ['small', 'default', 'large'] as const;

export interface Settings {
	/** The plain grey look instead of the colorful default. */
	focusMode: boolean;
	/** Color of buttons, focus rings and highlights. */
	accent: (typeof ACCENTS)[number];
	/** Corner rounding of cards, buttons and inputs; 'auto' is rounded, or square in Focus mode. */
	radius: (typeof RADII)[number];
	textSize: (typeof TEXT_SIZES)[number];
	/** Turn off animations, including the moving lines on the home map. */
	reduceMotion: boolean;
	/** The hint line at the bottom of the home map and the diagram. */
	canvasHints: boolean;
	/** Check GitHub for signed releases on launch and periodically while open. */
	automaticUpdates: boolean;
	/** The Getting started tour was shown (it opens by itself once, with the first database). */
	tourSeen: boolean;
}

/** Also read by the inline script in app.html, which applies these before first paint. */
export const SETTINGS_KEY = 'pixeldb.settings';

export const DEFAULT_SETTINGS: Settings = {
	focusMode: false,
	accent: 'neutral',
	radius: 'auto',
	textSize: 'default',
	reduceMotion: false,
	canvasHints: true,
	automaticUpdates: true,
	tourSeen: false
};

const pick = <T extends string>(options: readonly T[], value: unknown, fallback: T): T =>
	options.includes(value as T) ? (value as T) : fallback;

/** Fills defaults and drops anything unknown, so old or hand-edited storage can't break the app. */
export function readSettings(raw: unknown): Settings {
	const r = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>;
	const d = DEFAULT_SETTINGS;
	// Before Focus mode existed (no `version`), 'none' was the default rather than a choice.
	const radius = r.version === undefined && r.radius === 'none' ? 'auto' : r.radius;
	return {
		focusMode: typeof r.focusMode === 'boolean' ? r.focusMode : d.focusMode,
		accent: pick(ACCENTS, r.accent, d.accent),
		radius: pick(RADII, radius, d.radius),
		textSize: pick(TEXT_SIZES, r.textSize, d.textSize),
		reduceMotion: typeof r.reduceMotion === 'boolean' ? r.reduceMotion : d.reduceMotion,
		canvasHints: typeof r.canvasHints === 'boolean' ? r.canvasHints : d.canvasHints,
		automaticUpdates: typeof r.automaticUpdates === 'boolean' ? r.automaticUpdates : d.automaticUpdates,
		tourSeen: typeof r.tourSeen === 'boolean' ? r.tourSeen : d.tourSeen
	};
}

/** Stored with the settings so readSettings can tell old defaults from choices. */
export const SETTINGS_VERSION = 2;

/** The attributes on <html> that app.css keys the preferences on. */
export function settingsAttributes(s: Settings): Record<string, string | null> {
	return {
		'data-focus': s.focusMode ? '' : null,
		'data-ui-accent': s.accent === 'neutral' ? null : s.accent,
		'data-radius': s.radius === 'auto' ? null : s.radius,
		'data-text': s.textSize === 'default' ? null : s.textSize,
		'data-reduce-motion': s.reduceMotion ? '' : null
	};
}
