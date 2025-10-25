#!/usr/bin/env node

import chokidar from 'chokidar';
import { build } from 'esbuild';
import compress from 'esbuild-plugin-compress';
import { copy } from 'esbuild-plugin-copy';
import { createServer } from 'esbuild-server';
import { glob } from 'glob';
import { spawn } from 'node:child_process';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';
import { getConfig } from './config.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const viewerLibPath = path.join(__dirname, '..', 'viewer-lib');
const publicPath = path.join(__dirname, '..', 'public');

let rebuildingRust = false;

const shouldServe = process.argv[2] === '--server';

const config = {
	entryPoints: [
		'viewer-app/main.ts',
	],
	entryNames: '[name]',
	bundle: true,
	format: 'esm',
	loader: { '.wasm': 'file' },
	target: 'esnext',
};

if (shouldServe) {
	chokidar.watch(viewerLibPath, {
		ignored: [path.join(viewerLibPath, 'target')],
	}).on('change', rebuildRust);

	await rebuildRust();
	startServer();

} else {
	const appConfig = getConfig();
	console.log(`Building Frontend for ${appConfig.stage}`);

	await rebuildRust();
	await build({
		...config,
		write: false,
		outdir: 'dist',
		minify: true,
		sourcemap: false,
		platform: 'browser',
		plugins: [
			getCopyPlugin(),
			compress({ gzip: true }),
		],
	});
}

function startServer() {
	const server = createServer({
		...config,
		splitting: true,
		sourcemap: true,
		plugins: [getCopyPlugin()],
	}, {
		injectLiveReload: true,
		open: false,
		port: 8080,
		historyApiFallback: true,
		static: 'public',
	});

	server.start();
	console.log('🚀 Dev server running at http://localhost:8080');
}

function getCopyPlugin() {
	return copy({
		resolveFrom: 'cwd',
		assets: {
			from: ['./public/*'],
			to: ['./dist'],
		},
	});
}

/**
 * @returns Promise<void>
 */
function rebuildRust() {
	if (rebuildingRust) { return; }

	rebuildingRust = true;
	console.log('🔧 Rebuilding Rust…');

	const build = spawn('wasm-pack', [
		'build',
		'--target=web',
		'--no-default-features',
		'--release',
	], {
		env: { ...process.env, RUST_LOG: 'warn' },
		stdio: 'inherit',
		cwd: viewerLibPath,
	});

	let resolve = null;
	const promise = new Promise(r => { resolve = r; });
	build.on('exit', async function onExit() {
		rebuildingRust = false;
		console.log('✅ Rust build done');
		await copyWasm();
		resolve();
	});

	return promise;
}

async function copyWasm() {
	const files = await glob(path.join(viewerLibPath, 'pkg/small_world_viewer_bg.*'));
	await Promise.all(files.map(file =>
		fs.copyFile(
			file,
			path.join(publicPath, path.basename(file))
		)));
}
