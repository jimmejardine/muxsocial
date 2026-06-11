// Minimal service worker for installability + a basic offline fallback.
// Same-origin GETs are network-first (assets are content-hashed, so always prefer
// fresh), caching each response and falling back to the cache — then "/" — when
// offline. Cross-origin requests (network APIs, relays) and non-GET are left to the
// browser untouched.
const CACHE = "muxsocial-v1";

self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (event) => event.waitUntil(self.clients.claim()));

self.addEventListener("fetch", (event) => {
	const { request } = event;
	if (request.method !== "GET" || new URL(request.url).origin !== self.location.origin) return;
	event.respondWith(
		fetch(request)
			.then((response) => {
				const copy = response.clone();
				caches.open(CACHE).then((cache) => cache.put(request, copy));
				return response;
			})
			.catch(() => caches.match(request).then((cached) => cached ?? caches.match("/"))),
	);
});
