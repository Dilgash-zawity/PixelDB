import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, readdirSync, mkdtempSync, rmSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { spawnSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';

export function releaseVersion(root = '.') {
	const config = JSON.parse(readFileSync(join(root, 'src-tauri/tauri.conf.json'), 'utf8'));
	const pkg = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'));
	const cargo = readFileSync(join(root, 'src-tauri/Cargo.toml'), 'utf8');
	const cargoVersion = cargo.match(/\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m)?.[1];
	const lock = readFileSync(join(root, 'src-tauri/Cargo.lock'), 'utf8');
	const lockVersion = lock.match(/\[\[package\]\]\nname = "SlateDB"\nversion = "([^"]+)"/)?.[1];
	if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(config.version)) throw new Error('Use a stable x.y.z release version.');
	if (pkg.version !== config.version || cargoVersion !== config.version || lockVersion !== config.version) throw new Error('package.json, Cargo.toml, Cargo.lock and tauri.conf.json versions must match.');
	return config.version;
}

/** Build the manifest once, after all uploads, so concurrent jobs cannot lose platforms. */
export function createManifest(version, repo, files, signatureFor, notes = '') {
	if (!/^[\w.-]+\/[\w.-]+$/.test(repo)) throw new Error('Invalid repository.');
	const specs = [
		['darwin-aarch64', 'SlateDB_aarch64.app.tar.gz', 'app'],
		['darwin-x86_64', 'SlateDB_x64.app.tar.gz', 'app'],
		['windows-x86_64', `SlateDB_${version}_x64-setup.exe`, 'nsis'],
		['linux-x86_64', `SlateDB_${version}_amd64.AppImage`, 'appimage'],
		['linux-x86_64', `SlateDB_${version}_amd64.deb`, 'deb']
	];
	for (const name of [`SlateDB_${version}_aarch64.dmg`, `SlateDB_${version}_x64.dmg`]) {
		if (!files.has(name)) throw new Error(`Missing installer: ${name}`);
	}
	const platforms = {};
	for (const [platform, name, installer] of specs) {
		if (!files.has(name) || !files.has(`${name}.sig`)) throw new Error(`Missing signed updater: ${name}`);
		const signature = signatureFor(`${name}.sig`).trim();
		const comment = Buffer.from(signature, 'base64').toString('utf8').split('\n').find((line) => line.startsWith('trusted comment: '));
		if (!comment?.slice('trusted comment: '.length).split('\t').includes(`version:${version}`)) {
			throw new Error(`Signature does not bind release version: ${name}`);
		}
		const entry = { signature, url: `https://github.com/${repo}/releases/download/v${version}/${encodeURIComponent(name)}` };
		platforms[`${platform}-${installer}`] = entry;
		platforms[platform] ??= entry;
	}
	return { version, notes, pub_date: new Date().toISOString(), platforms };
}

function prepareAssets(folder, version, repo, notesFile = 'docs/release-notes.md') {
	if (version !== releaseVersion()) throw new Error('Release assets must match the checked-out version.');
	const files = new Set(readdirSync(folder));
	const manifest = createManifest(version, repo, files, (name) => readFileSync(join(folder, name), 'utf8'), readFileSync(notesFile, 'utf8'));
	const config = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'));
	const verificationDir = mkdtempSync(join(tmpdir(), 'slatedb-verify-'));
	try {
		const publicKey = Buffer.from(config.plugins.updater.pubkey, 'base64');
		writeFileSync(join(verificationDir, 'updater.pub'), publicKey);
		for (const name of files) {
			if (!name.endsWith('.sig')) continue;
			const artifact = name.slice(0, -4);
			if (!files.has(artifact)) throw new Error(`Signature has no artifact: ${name}`);
			writeFileSync(join(verificationDir, 'signature'), Buffer.from(readFileSync(join(folder, name), 'utf8').trim(), 'base64'));
			const result = spawnSync('minisign', ['-Vm', resolve(folder, artifact), '-p', join(verificationDir, 'updater.pub'), '-x', join(verificationDir, 'signature')]);
			if (result.status !== 0) throw new Error(`Signature verification failed: ${artifact}`);
		}
		writeFileSync(join(folder, 'updater.pub'), publicKey);
		writeFileSync(join(folder, 'latest.json'), JSON.stringify(manifest, null, 2) + '\n');
		const checksums = readdirSync(folder).filter((name) => name !== 'SHA256SUMS').sort().map((name) => {
			const digest = createHash('sha256').update(readFileSync(join(folder, name))).digest('hex');
			return `${digest}  ${name}`;
		});
		writeFileSync(join(folder, 'SHA256SUMS'), checksums.join('\n') + '\n');
	} finally {
		rmSync(verificationDir, { recursive: true, force: true });
	}
}

export function windowsConfig(env) {
	const { AZURE_SIGNING_ENDPOINT: endpoint, AZURE_SIGNING_ACCOUNT: account, AZURE_CERTIFICATE_PROFILE: profile } = env;
	if (!/^https:\/\/[a-z0-9-]+\.codesigning\.azure\.net\/?$/.test(endpoint ?? '')) throw new Error('Invalid Azure signing endpoint.');
	if (![account, profile].every((value) => /^[a-zA-Z0-9-]+$/.test(value ?? ''))) throw new Error('Invalid Azure signing account or profile.');
	return { bundle: { windows: { signCommand: `artifact-signing-cli -e ${endpoint} -a ${account} -c ${profile} -d SlateDB %1` } } };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
	const [command, ...args] = process.argv.slice(2);
	if (command === 'version') console.log(releaseVersion());
	else if (command === 'assets') prepareAssets(...args);
	else if (command === 'windows-config') writeFileSync('tauri-signing.windows.json', JSON.stringify(windowsConfig(process.env)));
	else throw new Error('Usage: release.mjs version | assets <folder> <version> <owner/repo> [notes-file] | windows-config');
}
