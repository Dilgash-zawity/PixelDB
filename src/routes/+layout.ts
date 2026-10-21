// Client-side only: Tauri serves the static build. Prerendering emits an
// HTML shell per route so reloads resolve without relying on the fallback.
export const ssr = false;
export const prerender = true;
