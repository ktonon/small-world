import init, { Globe } from "./small_world_viewer.js";

const ZOOM_SPEED = 0.02;
const ROT_SPEED = 0.005;
const MAX_DIST = 5;
const MIN_DIST = 1.2;

(async () => {
	await init();
	const canvas = document.getElementById("globe");
	canvas.width = canvas.clientWidth * window.devicePixelRatio;
	canvas.height = canvas.clientHeight * window.devicePixelRatio;

	const globe = new Globe(canvas);
	setupControls(canvas, globe);
	enableTouchGestures(canvas, globe);

	const useVideo = true;
	if (useVideo) {
		const video = document.getElementById("earthVideo");
		await video.play(); // ensure it’s decoded and running

		globe.set_image_video(video);
		function frame() {
			resizeCanvasToDisplaySize(canvas);
			globe.set_image_video(video); // updates the texture with the current frame
			globe.render();
			video.requestVideoFrameCallback(frame);
		}
		video.requestVideoFrameCallback(frame);

	} else {
		globe.set_image(await loadImage('../images/age.2020.1.GTS2012.png'));
		function frame() {
			resizeCanvasToDisplaySize(canvas);
			globe.render();
			requestAnimationFrame(frame);
		}
		requestAnimationFrame(frame);
	}
})();

async function loadImage(url) {
	const resp = await fetch(url);
	if (!resp.ok) throw new Error("Failed to fetch image");

	const blob = await resp.blob();
	console.log("blob size:", blob.size, "type:", blob.type);

	const image = await createImageBitmap(blob);
	console.log("Image bitmap:", image.width, 'x', image.height);

	return image;
}

function resizeCanvasToDisplaySize(canvas) {
	const dpr = window.devicePixelRatio || 1;
	const displayWidth = Math.floor(canvas.clientWidth * dpr);
	const displayHeight = Math.floor(canvas.clientHeight * dpr);

	if (canvas.width !== displayWidth || canvas.height !== displayHeight) {
		canvas.width = displayWidth;
		canvas.height = displayHeight;
	}
}

function setupControls(canvas, globe) {
	if (isTouchCapable()) { return; }

	let dragging = false;
	let lastX = 0, lastY = 0;
	let dist = 2.2;
	const TWIST_KEY = 0.06; // radians per key press

	canvas.addEventListener("wheel", e => {
		e.preventDefault();
		const k = 1.0 - Math.sign(e.deltaY) * ZOOM_SPEED; // zoom step
		dist = Math.min(MAX_DIST, Math.max(MIN_DIST, dist * k));
		globe.set_distance(dist);
	}, { passive: false });

	window.addEventListener("keydown", e => {
		if (e.target !== document.body) return; // avoid typing fields
		if (e.key === "q" || e.key === "Q") { globe.apply_twist(-TWIST_KEY); }
		if (e.key === "e" || e.key === "E") { globe.apply_twist(+TWIST_KEY); }
	});

	canvas.addEventListener("pointerdown", e => {
		dragging = true;
		lastX = e.clientX;
		lastY = e.clientY;
		canvas.setPointerCapture(e.pointerId);
	});

	canvas.addEventListener("pointermove", e => {
		if (!dragging) return;
		const dx = e.clientX - lastX;
		const dy = e.clientY - lastY;
		lastX = e.clientX;
		lastY = e.clientY;

		const rotSpeed = ROT_SPEED * dist / 5;
		globe.apply_drag(dx, dy, rotSpeed);
	});

	canvas.addEventListener("pointerup", e => {
		dragging = false;
		canvas.releasePointerCapture(e.pointerId);
	});
}

function isTouchCapable() {
	// Prefer Pointer Events + maxTouchPoints, fallback to older signals
	return (window.PointerEvent && navigator.maxTouchPoints > 0)
		|| (window.matchMedia?.('(hover: none) and (pointer: coarse)').matches)
		|| ('ontouchstart' in window);
}

function enableTouchGestures(canvas, globe, { minDist = MIN_DIST, maxDist = MAX_DIST } = {}) {
	if (!isTouchCapable()) { return; } // do nothing on non-touch

	const touches = new Map();
	let dragging = false;
	let lastX = 0, lastY = 0;
	let twistLast = null;
	let pinchStart = 0, pinchDist = 0;
	let dist = 2.2; // keep your current dist source if stored elsewhere

	function onDown(e) {
		touches.set(e.pointerId, e);
		if (touches.size === 1) {
			dragging = true;
			lastX = e.clientX;
			lastY = e.clientY;
		} else if (touches.size === 2) {
			const [p1, p2] = [...touches.values()];
			twistLast = vecAngle(p1, p2);
			pinchStart = pinchLen(p1, p2);
			pinchDist = dist;
		}
		canvas.setPointerCapture(e.pointerId);
	}

	function onMove(e) {
		touches.set(e.pointerId, e);
		if (touches.size === 1 && dragging) {
			const dx = e.clientX - lastX;
			const dy = e.clientY - lastY;
			lastX = e.clientX;
			lastY = e.clientY;

			const rotSpeed = ROT_SPEED * dist / 5;
			globe.apply_drag(dx, dy, rotSpeed);

		} else if (touches.size === 2) {
			const [p1, p2] = [...touches.values()];

			// twist (roll)
			const ang = vecAngle(p1, p2);
			if (twistLast != null) {
				let delta = ang - twistLast;
				if (delta > Math.PI) delta -= 2 * Math.PI;
				if (delta < -Math.PI) delta += 2 * Math.PI;
				globe.apply_twist(delta); // your Rust method
			}
			twistLast = ang;

			// pinch (zoom)
			const len = pinchLen(p1, p2);
			if (pinchStart) {
				const factor = pinchStart / len;
				dist = Math.min(maxDist, Math.max(minDist, pinchDist * factor));
				globe.set_distance(dist);
			}
		}
	}

	function onEnd(e) {
		touches.delete(e.pointerId);
		if (touches.size < 2) {
			twistLast = null;
			pinchStart = 0;
		}
		canvas.releasePointerCapture(e.pointerId);
	}

	canvas.addEventListener('pointerdown', onDown);
	canvas.addEventListener('pointermove', onMove);
	canvas.addEventListener('pointerup', onEnd);
	canvas.addEventListener('pointercancel', onEnd);
}

function vecAngle(a, b) {
	return Math.atan2(a.clientY - b.clientY, a.clientX - b.clientX);
}

function pinchLen(a, b) {
	return Math.hypot(a.clientX - b.clientX, a.clientY - b.clientY);
}
