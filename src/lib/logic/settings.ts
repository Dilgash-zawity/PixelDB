/** Look-and-feel preferences for this computer, kept in the webview's local storage. */

export const ACCENTS = ['neutral', 'blue', 'green', 'amber', 'red', 'violet'] as const;
export const RADII = ['none', 'small', 'medium', 'large'] as const;
export const TEXT_SIZES = ['small', 'default', 'large'] as const;

export interface Settings {
	/** Color of buttons, focus rings and highlights. */
	accent: (typeof ACCENTS)[number];
	/** Corner rounding of cards, buttons and inputs. */
	radius: (typeof RADII)[number];
	textSize: (typeof TEXT_SIZES)[number];
	/** Turn off animations, including the moving lines on the home map. */
	reduceMotion: boolean;
	/** The hint line at the bottom of the home map and the diagram. */
	canvasHints: boolean;
}

/** Also read by the inline script in app.html, which applies these before first paint. */
export const SETTINGS_KEY = 'slatedb.settings';

export const DEFAULT_SETTINGS: Settings = {
	accent: 'neutral',
	radius: 'none',
	textSize: 'default',
	reduceMotion: false,
	canvasHints: true
};

const pick = <T extends string>(options: readonly T[], value: unknown, fallback: T): T =>
	options.includes(value as T) ? (value as T) : fallback;

/** Fills defaults and drops anything unknown, so old or hand-edited storage can't break the app. */
export function readSettings(raw: unknown): Settings {
	const r = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>;
	const d = DEFAULT_SETTINGS;
	return {
		accent: pick(ACCENTS, r.accent, d.accent),
		radius: pick(RADII, r.radius, d.radius),
		textSize: pick(TEXT_SIZES, r.textSize, d.textSize),
		reduceMotion: typeof r.reduceMotion === 'boolean' ? r.reduceMotion : d.reduceMotion,
		canvasHints: typeof r.canvasHints === 'boolean' ? r.canvasHints : d.canvasHints
	};
}

/** The attributes on <html> that app.css keys the preferences on. */
export function settingsAttributes(s: Settings): Record<string, string | null> {
	return {
		'data-ui-accent': s.accent === 'neutral' ? null : s.accent,
		'data-radius': s.radius === 'none' ? null : s.radius,
		'data-text': s.textSize === 'default' ? null : s.textSize,
		'data-reduce-motion': s.reduceMotion ? '' : null
	};
}
