#!/usr/bin/env node

import { CloudFrontClient, CreateInvalidationCommand } from '@aws-sdk/client-cloudfront';
import { DeleteObjectCommand, ListObjectsV2Command, PutObjectCommand, S3Client } from '@aws-sdk/client-s3';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { getConfig } from './config.js';

const config = getConfig();
const s3Client = new S3Client({ region: config.region });
const Bucket = config.bucket;

await cleanBucket();
await copyAppToS3();
await invalidateDistro();

async function cleanBucket() {
	const { Contents } = await s3Client.send(new ListObjectsV2Command({ Bucket }));
	const keys = (Contents || []).map(c => c.Key);
	await Promise.all(keys.map(Key => s3Client.send(new DeleteObjectCommand({ Bucket, Key }))));
}

async function copyAppToS3() {
	const __dirname = path.dirname(fileURLToPath(import.meta.url));
	const root = path.join(__dirname, '..');
	const dist = path.join(root, 'dist');
	const files = await fs.readdir(dist);

	console.log(`Uploading:\n  - ${files.join('\n  - ')} for ${config.stage} to ${Bucket}`);

	await Promise.all(files.map(async filename => {
		const Key = filename;
		const filepath = path.join(dist, filename);
		const Body = await fs.readFile(filepath);

		try {
			await s3Client.send(new PutObjectCommand({
				Body,
				Bucket,
				ContentDisposition: 'inline',
				ContentType: mimeFromFilename(filename),
				Key,
				...filename.endsWith('.gz') && {
					ContentEncoding: 'gzip',
				},
				...filename.endsWith('.br') && {
					ContentEncoding: 'br',
				},
				...filename.endsWith('.webm') && {
					ContentType: 'video/webm',
				},
				...filename.endsWith('.webp') && {
					ContentType: 'image/webp',
				},
			}));
		} catch (error) {
			console.error('error', error);
		}
	}));
}

function mimeFromFilename(filename) {
	return filename.endsWith('.html') ? 'text/html' : 'application/javascript';
}

async function invalidateDistro() {
	const client = new CloudFrontClient();
	console.log(`Invalidating: ${config.distro}`);
	await client.send(new CreateInvalidationCommand({
		DistributionId: config.distro,
		InvalidationBatch: {
			CallerReference: `invalidate-${Date.now()}`,
			Paths: {
				Items: ['/*'],
				Quantity: 1,
			},
		},
	}));
}
