/** Native folder picker. Returns null when cancelled. */
export async function pickFolder(): Promise<string | null> {
	const { open } = await import('@tauri-apps/plugin-dialog');
	const picked = await open({ title: 'Choose a folder for your databases', directory: true, multiple: false });
	return typeof picked === 'string' ? picked : null;
}
