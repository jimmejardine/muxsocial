/**
 * Dedicated Web Worker that owns the {@link MuxsocialClientWasm} instance.
 *
 * The main thread never loads WASM directly — it communicates with this
 * worker via `MessageChannel` RPC (see {@link MuxsocialWorkerRPC} for the
 * message types).  On startup the worker initialises WASM, then signals
 * readiness to the main thread.
 *
 * Every method call on `MuxsocialClientWasmProxy` in the main thread arrives
 * here as a `{ type: "call", method, args }` message, is dispatched to
 * the WASM client, and the result (or error) is posted back.
 *
 * @module
 */

import { MuxsocialClientWasm, wasm_init } from "../../../muxsocial-rust/muxsocial-client-wasm/pkg";
import type { RPCRequest, RPCResponse } from "../tools/MuxsocialWorkerRPC.ts";

let muxsocial_client: MuxsocialClientWasm | null = null;

function assertClient(): MuxsocialClientWasm {
	if (!muxsocial_client) {
		throw new Error("MuxsocialClientWasm not initialized. Call create first.");
	}
	return muxsocial_client;
}

function isExposedMethodName(method_name: string) {
	// Prevent calling lifecycle internals from UI
	return method_name !== "constructor" && method_name !== "free" && method_name !== "dispose";
}

function postOk(port: MessagePort, result: unknown) {
	const rpc_response: RPCResponse = { ok: true, result };
	port.postMessage(rpc_response);
}

function postErr(port: MessagePort, err: unknown) {
	const error_object = err instanceof Error ? err : new Error(String(err));
	const rpc_response: RPCResponse = {
		ok: false,
		error: { name: error_object.name, message: error_object.message, stack: error_object.stack },
	};
	port.postMessage(rpc_response);
}

const onMessage = async (event: MessageEvent<RPCRequest>) => {
	const rpc_request = event.data;
	const reply_port = event.ports[0];
	if (!reply_port) {
		console.error("MuxsocialWorker: received message without a reply port, ignoring:", rpc_request);
		return;
	}

	try {
		if (rpc_request.type === "create") {
			muxsocial_client = await MuxsocialClientWasm.create_new();
			postOk(reply_port, true);
			return;
		}

		if (rpc_request.type === "dispose") {
			if (muxsocial_client) {
				muxsocial_client.free();
				muxsocial_client = null;
			}
			postOk(reply_port, true);
			return;
		}

		if (rpc_request.type === "call") {
			const muxsocial_client_instance = assertClient();

			if (!isExposedMethodName(rpc_request.method)) {
				postErr(reply_port, new Error(`Method not exposed: ${rpc_request.method}`));
				return;
			}

			const wasm_method = (muxsocial_client_instance as unknown as Record<string, unknown>)[rpc_request.method];
			if (typeof wasm_method !== "function") {
				postErr(reply_port, new Error(`No such method on MuxsocialClientWasm: ${rpc_request.method}`));
				return;
			}

			const wasm_method_result = await wasm_method.apply(muxsocial_client_instance, rpc_request.args);
			postOk(reply_port, wasm_method_result);
			return;
		}
	} catch (err) {
		postErr(reply_port, err);
	}
};

wasm_init(true);

// Serialise message handling: each request fully completes before the next
// starts. The WASM client's `&mut self` command methods would panic with
// "recursive use of an object" if two async calls overlapped, which a plain
// async `onmessage` allows. Chaining them keeps mutations ordered and safe.
let pending_chain: Promise<void> = Promise.resolve();
self.onmessage = (event: MessageEvent<RPCRequest>) => {
	pending_chain = pending_chain.then(() => onMessage(event));
};

self.postMessage({ type: "ready" });
