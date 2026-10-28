import { describe, expect, test } from 'bun:test';
import { createManifest, releaseVersion, windowsConfig } from './release.mjs';

const version = '0.0.10';
const installers = [
	`PixelDB_${version}_aarch64.dmg`, `PixelDB_${version}_x64.dmg`,
	'PixelDB_aarch64.app.tar.gz', 'PixelDB_x64.app.tar.gz',
	`PixelDB_${version}_x64-setup.exe`, `PixelDB_${version}_amd64.AppImage`, `PixelDB_${version}_amd64.deb`
];
const files = () => new Set([...installers, ...installers.slice(2).map((name) => `${name}.sig`)]);
const signature = () => Buffer.from(`untrusted comment: test\nsignature\ntrusted comment: timestamp:1\tfile:app\tversion:${version}\nglobal\n`).toString('base64');

describe('release validation', () => {
	test('the project versions agree', () => {
		expect(releaseVersion()).toMatch(/^\d+\.\d+\.\d+$/);
	});
	test('each installer has the correct updater payload, including Debian', () => {
		const manifest = createManifest(version, 'Dilgash-zawity/PixelDB', files(), signature);
		expect(manifest.platforms['linux-x86_64-deb'].url).toEndWith('.deb');
		expect(manifest.platforms['linux-x86_64'].url).toEndWith('.AppImage');
		expect(manifest.platforms['darwin-aarch64'].url).toEndWith('aarch64.app.tar.gz');
		expect(manifest.platforms['windows-x86_64-nsis'].signature).toBe(signature());
	});
	test('a partial platform upload cannot publish', () => {
		const missing = files(); missing.delete('PixelDB_x64.app.tar.gz.sig');
		expect(() => createManifest(version, 'Dilgash-zawity/PixelDB', missing, signature)).toThrow('Missing signed updater');
	});
	test('an older signed artifact cannot be labelled as a newer release', () => {
		expect(() => createManifest(version, 'Dilgash-zawity/PixelDB', files(), () => Buffer.from('trusted comment: version:0.0.9').toString('base64'))).toThrow('Signature does not bind release version');
	});
	test('signing configuration rejects shell syntax and non-Azure endpoints', () => {
		const env = { AZURE_SIGNING_ENDPOINT: 'https://eus.codesigning.azure.net', AZURE_SIGNING_ACCOUNT: 'pixeldb', AZURE_CERTIFICATE_PROFILE: 'public-trust' };
		expect(windowsConfig(env).bundle.windows.signCommand).toContain('-d PixelDB %1');
		expect(() => windowsConfig({ ...env, AZURE_SIGNING_ACCOUNT: 'a;echo secret' })).toThrow();
		expect(() => windowsConfig({ ...env, AZURE_SIGNING_ENDPOINT: 'http://example.com' })).toThrow();
	});
});
