/**
 * Message types for the `MessageChannel` RPC between the main-thread
 * {@link Muxsocial} facade and the {@link MuxsocialWorker}.
 *
 * @module
 */

export type RPCRequest = { type: "create" } | { type: "call"; method: string; args: unknown[] } | { type: "dispose" };

export type RPCResponse = { ok: true; result: unknown } | { ok: false; error: { message: string; name?: string; stack?: string } };
