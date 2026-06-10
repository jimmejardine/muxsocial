/**
 * Main-thread API facade for the muxsocial WASM client.
 *
 * All WASM work runs off the main thread inside a dedicated
 * {@link MuxsocialWorker}.  This module spawns that worker, waits for
 * it to initialise, and returns a {@link MuxsocialClientWasmProxy} proxy whose
 * method calls are transparently forwarded over `MessageChannel` RPC.
 *
 * UI code should only interact with muxsocial through {@link MuxsocialClientWasmProxy}
 * — it never touches WASM or the worker directly.
 *
 * @module
 */

import type { MuxsocialClientWasm } from "../../muxsocial-rust/muxsocial-client-wasm/pkg";
import { DeferredPromise } from "./tools/DeferredPromise.ts";
import type { RPCRequest, RPCResponse } from "./tools/MuxsocialWorkerRPC.ts";

// biome-ignore lint/suspicious/noExplicitAny: required for conditional type inference in MethodKeys/WrapAsync
type AnyFn = (...args: any[]) => any;

type MethodKeys<T> = {
	[K in keyof T]: T[K] extends AnyFn ? K : never;
}[keyof T];

// biome-ignore lint/suspicious/noExplicitAny: required for conditional type inference in MethodKeys/WrapAsync
type WrapAsync<T extends AnyFn> = T extends (...args: infer A) => infer R ? (R extends Promise<any> ? T : (...args: A) => Promise<R>) : never;

/**
 * Async proxy type mirroring every public method on {@link MuxsocialClientWasm},
 * with all return types wrapped in `Promise`.  Lifecycle methods (`free`,
 * `Symbol.dispose`) are excluded and replaced with a JS-friendly `dispose()`.
 *
 * Calls on this proxy are forwarded over `MessageChannel` RPC to the
 * {@link MuxsocialWorker}, which invokes the real {@link MuxsocialClientWasm}
 * method inside its dedicated Web Worker.
 */
export type MuxsocialClientWasmProxy = {
	[K in Exclude<MethodKeys<MuxsocialClientWasm>, "free" | typeof Symbol.dispose>]: WrapAsync<MuxsocialClientWasm[K]>;
} & { dispose(): Promise<void> };

// Memoised so React StrictMode's double-mounted effects don't spawn two workers.
let muxsocial_create_promise: Promise<MuxsocialClientWasmProxy> | null = null;

export class Muxsocial {
	static create(): Promise<MuxsocialClientWasmProxy> {
		if (!muxsocial_create_promise) {
			muxsocial_create_promise = Muxsocial.create_uncached();
		}
		return muxsocial_create_promise;
	}

	private static async create_uncached(): Promise<MuxsocialClientWasmProxy> {
		const worker_ready_deferred_promise = new DeferredPromise<null>();

		console.log("Creating Muxsocial worker");
		const worker = new Worker(new URL("./workers/MuxsocialWorker.ts", import.meta.url), { type: "module", name: "MuxsocialWorker" });
		console.log("Created Muxsocial worker");

		// Wait for the worker to signal it's ready (one-time bootstrap via self.postMessage)
		worker.onmessage = async () => {
			worker.onmessage = null;
			await rpc<boolean>({ type: "create" });
			worker_ready_deferred_promise.resolve(null);
		};

		worker.onerror = (e) => {
			console.error("Muxsocial worker error:", e.message);
		};

		function rpc<T>(request: RPCRequest): Promise<T> {
			return new Promise<T>((resolve, reject) => {
				const channel = new MessageChannel();
				channel.port1.onmessage = (event: MessageEvent<RPCResponse>) => {
					channel.port1.close();
					const response = event.data;
					if (response.ok) {
						resolve(response.result as T);
					} else {
						const err = new Error(response.error.message);
						err.name = response.error.name ?? "WorkerError";
						(err as unknown as Record<string, unknown>).stack = response.error.stack;
						reject(err);
					}
				};
				worker.postMessage(request, [channel.port2]);
			});
		}

		await worker_ready_deferred_promise.promise;

		const method_cache = new Map<string | symbol, unknown>();

		const api = new Proxy(
			{},
			{
				get(_target, prop) {
					const cached = method_cache.get(prop);
					if (cached !== undefined) return cached;

					let value: unknown;

					if (prop === "dispose") {
						value = async () => {
							await rpc({ type: "dispose" });
							worker.terminate();
							muxsocial_create_promise = null;
						};
					} else if (prop === "then" || prop === "catch" || prop === "finally" || prop === "toJSON") {
						return undefined;
					} else if (typeof prop !== "string") {
						return undefined;
					} else {
						value = (...args: unknown[]) => rpc({ type: "call", method: prop, args });
					}

					method_cache.set(prop, value);
					return value;
				},
			},
		);

		return api as MuxsocialClientWasmProxy;
	}
}
