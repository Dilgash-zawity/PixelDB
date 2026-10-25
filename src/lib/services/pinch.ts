import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { PinchTracker, pinchDelta } from '../logic/pinch';

/**
 * Touchpad pinch-to-zoom for the canvases (home map, diagrams): a pinch is
 * replayed as Ctrl+wheel under the pointer, which they already zoom on.
 * Linux gets pinches from the window (`touchpad-pinch`, see `touchpad.rs`);
 * macOS's WebKit sends gesture events; Windows sends Ctrl+wheel itself.
 * Returns a function that stops listening.
 */
export function startPinchZoom(): () => void {
	let pointer = { x: innerWidth / 2, y: innerHeight / 2 };
	/** When the platform last sent a real Ctrl+wheel, so a pinch isn't zoomed twice. */
	let nativeWheelAt = 0;
	const tracker = new PinchTracker();

	const zoomBy = (ratio: number | null) => {
		if (ratio === null) return;
		const target = document.elementFromPoint(pointer.x, pointer.y) ?? document.body;
		target.dispatchEvent(
			new WheelEvent('wheel', { deltaY: pinchDelta(ratio), ctrlKey: true, clientX: pointer.x, clientY: pointer.y, bubbles: true, cancelable: true })
		);
	};

	const onPointer = (e: PointerEvent) => (pointer = { x: e.clientX, y: e.clientY });
	const onWheel = (e: WheelEvent) => {
		if (e.isTrusted && e.ctrlKey) nativeWheelAt = Date.now();
	};
	// Safari-style gesture events (macOS); not in the DOM typings.
	type GestureEvent = Event & { scale: number; clientX: number; clientY: number };
	const onGestureStart = (e: Event) => {
		e.preventDefault();
		tracker.begin();
	};
	const onGestureChange = (e: Event) => {
		e.preventDefault();
		const g = e as GestureEvent;
		if (Number.isFinite(g.clientX)) pointer = { x: g.clientX, y: g.clientY };
		const ratio = tracker.step(g.scale);
		if (Date.now() - nativeWheelAt > 200) zoomBy(ratio);
	};

	addEventListener('pointermove', onPointer, { passive: true });
	addEventListener('wheel', onWheel, { passive: true, capture: true });
	addEventListener('gesturestart', onGestureStart);
	addEventListener('gesturechange', onGestureChange);

	let unlisten: (() => void) | null = null;
	let stopped = false;
	if (isTauri()) {
		listen<{ phase: 'begin' | 'update' | 'end' | 'cancel'; scale: number }>('touchpad-pinch', ({ payload }) => {
			if (payload.phase === 'begin') tracker.begin();
			else if (payload.phase === 'update') zoomBy(tracker.step(payload.scale));
		})
			.then((u) => (stopped ? u() : (unlisten = u)))
			.catch(() => {
				// Not in the desktop app (e.g. a plain browser): nothing to forward.
			});
	}

	return () => {
		stopped = true;
		unlisten?.();
		removeEventListener('pointermove', onPointer);
		removeEventListener('wheel', onWheel, { capture: true });
		removeEventListener('gesturestart', onGestureStart);
		removeEventListener('gesturechange', onGestureChange);
	};
}
