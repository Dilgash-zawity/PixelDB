/**
 * Turning a pinch into the Ctrl+wheel zoom the canvases understand: they
 * zoom by `exp(-deltaY * WHEEL_ZOOM_RATE)` per wheel event.
 */
export const WHEEL_ZOOM_RATE = 0.01;

/** The wheel `deltaY` that zooms by `ratio` (2 = twice as big). */
export function pinchDelta(ratio: number): number {
	if (!(ratio > 0) || !Number.isFinite(ratio)) return 0;
	return -Math.log(ratio) / WHEEL_ZOOM_RATE;
}

/**
 * Follows one pinch whose scale is measured from where it began (as both
 * GTK and Safari report it) and gives the change since the last step.
 */
export class PinchTracker {
	private last = 1;

	begin() {
		this.last = 1;
	}

	/** The zoom ratio since the previous step, or null for no change. */
	step(scale: number): number | null {
		if (!(scale > 0) || !(this.last > 0)) return null;
		const ratio = scale / this.last;
		this.last = scale;
		return ratio === 1 ? null : ratio;
	}
}
