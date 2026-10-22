import { DEFAULT_SETTINGS, SETTINGS_KEY, readSettings, settingsAttributes, type Settings } from '../logic/settings';

class SettingsState {
	current = $state<Settings>(load());

	update(patch: Partial<Settings>) {
		this.current = { ...this.current, ...patch };
		this.persist();
	}

	reset() {
		this.current = { ...DEFAULT_SETTINGS };
		this.persist();
	}

	/** Mirrors the settings onto <html>. */
	apply() {
		if (typeof document === 'undefined') return;
		for (const [name, value] of Object.entries(settingsAttributes(this.current))) {
			if (value === null) document.documentElement.removeAttribute(name);
			else document.documentElement.setAttribute(name, value);
		}
	}

	private persist() {
		try {
			localStorage.setItem(SETTINGS_KEY, JSON.stringify(this.current));
		} catch {
			// Unavailable storage just means the choice lasts until restart.
		}
	}
}

function load(): Settings {
	try {
		return readSettings(JSON.parse(localStorage.getItem(SETTINGS_KEY) ?? 'null'));
	} catch {
		return { ...DEFAULT_SETTINGS };
	}
}

export const settings = new SettingsState();
