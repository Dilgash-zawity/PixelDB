/**
 * Translations. Text shown to people lives in catalogs (`en.ts`, and one file
 * per added language with the same keys), looked up with `t('area.key')`.
 *
 * - `{name}` in a message is replaced by `params.name`.
 * - A message can be plural forms (`{ one: '{count} record', other: '{count} records' }`);
 *   `params.count` picks the form by the language's plural rules.
 * - A key missing from the current language falls back to English, and a key
 *   missing from English is shown as itself, so a gap is visible but never breaks a screen.
 */
import { en } from './en';

/** Plural forms, as Intl.PluralRules names them; `other` is always required. */
export type Plural = { other: string } & Partial<Record<'zero' | 'one' | 'two' | 'few' | 'many', string>>;
export type Message = string | Plural;
export interface Catalog {
	[key: string]: Message | Catalog;
}

type Leaves<T, P extends string = ''> = {
	[K in keyof T & string]: T[K] extends string ? `${P}${K}` : T[K] extends { other: string } ? `${P}${K}` : Leaves<T[K], `${P}${K}.`>;
}[keyof T & string];

/** Every key of the English catalog, so a typo is a type error. */
export type MessageKey = Leaves<typeof en>;
export type Params = Record<string, string | number>;

/** A catalog with the same shape as English (every leaf a message). */
export type Translation = { [K in keyof typeof en]?: unknown };

const catalogs: Record<string, Catalog> = { en };
let locale = 'en';

/** Adds a language's catalog (see `missingKeys` to check it is complete). */
export function addCatalog(code: string, catalog: Catalog): void {
	catalogs[code] = catalog;
}

/** Uses `code` when its catalog exists (else English); returns the language in use. */
export function setLocale(code: string): string {
	const base = code.split('-')[0];
	locale = catalogs[code] ? code : catalogs[base] ? base : 'en';
	return locale;
}

export function currentLocale(): string {
	return locale;
}

function lookup(catalog: Catalog | undefined, key: string): Message | undefined {
	let node: Message | Catalog | undefined = catalog;
	for (const part of key.split('.')) {
		if (!node || typeof node === 'string') return undefined;
		node = (node as Catalog)[part];
	}
	if (typeof node === 'string') return node;
	if (node && typeof node === 'object' && typeof node.other === 'string') return node as Plural;
	return undefined;
}

const rules = new Map<string, Intl.PluralRules>();
function pluralForm(lang: string, count: number): Intl.LDMLPluralRule {
	let r = rules.get(lang);
	if (!r) rules.set(lang, (r = new Intl.PluralRules(lang)));
	return r.select(count);
}

/** Fills `{name}` placeholders; unknown ones stay as written. */
export function format(text: string, params: Params = {}): string {
	return text.replace(/\{(\w+)\}/g, (all, name: string) => (name in params ? String(params[name]) : all));
}

/** The text of `key` in the current language. */
export function t(key: MessageKey, params?: Params): string {
	let lang = locale;
	let msg = lookup(catalogs[lang], key);
	if (msg === undefined && lang !== 'en') {
		lang = 'en';
		msg = lookup(en as Catalog, key);
	}
	if (msg === undefined) return key;
	if (typeof msg === 'string') return format(msg, params);
	const count = Number(params?.count ?? 0);
	const form = msg[pluralForm(lang, count)] ?? msg.other;
	return format(form, { ...params, count: params?.count !== undefined ? count.toLocaleString(lang) : '' });
}

/** Keys English has that `catalog` lacks, for tests of added languages. */
export function missingKeys(catalog: Catalog, reference: Catalog = en as Catalog, prefix = ''): string[] {
	const out: string[] = [];
	for (const [k, v] of Object.entries(reference)) {
		const key = prefix + k;
		const other = catalog[k];
		if (typeof v === 'string' || (v && typeof (v as Plural).other === 'string')) {
			if (other === undefined) out.push(key);
		} else if (other && typeof other === 'object') out.push(...missingKeys(other as Catalog, v as Catalog, `${key}.`));
		else out.push(key);
	}
	return out;
}
