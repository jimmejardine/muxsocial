# Worker RPC

All WASM work runs off the main thread inside a dedicated Web Worker (`MuxsocialWorker`). UI code never touches WASM or the worker directly — it only interacts with `MuxsocialClientWasmProxy`.

## Components

- `src/Muxsocial.ts` — main-thread facade. `Muxsocial.create()` spawns the worker, waits for its one-time `{ type: "ready" }` bootstrap message, sends `{ type: "create" }` to construct the WASM client inside the worker, and returns a `Proxy` whose property accesses become RPC method calls. The create promise is memoised at module level so React StrictMode's double-mounted effects don't spawn two workers.
- `src/workers/MuxsocialWorker.ts` — owns the single `MuxsocialClientWasm` instance. Calls `wasm_init(true)` on startup, then dispatches incoming requests.
- `src/tools/MuxsocialWorkerRPC.ts` — the request/response message types shared by both sides.

## Protocol

Each call creates a fresh `MessageChannel`; the request is posted to the worker with `port2` transferred, and the response arrives on `port1`. This gives every in-flight call its own reply channel — no correlation IDs needed.

Requests:

```ts
{ type: "create" }                                    // construct MuxsocialClientWasm inside the worker
{ type: "call"; method: string; args: unknown[] }     // invoke a method on the WASM client
{ type: "dispose" }                                   // free the WASM client; the facade then terminates the worker
```

Responses:

```ts
{ ok: true; result: unknown }
{ ok: false; error: { message: string; name?: string; stack?: string } }
```

Errors thrown inside the worker are serialised into the `error` shape and re-hydrated as `Error` objects on the main thread.

## Typing

`MuxsocialClientWasmProxy` is derived from the wasm-bindgen-generated `MuxsocialClientWasm` type: every public method is mirrored with its return type wrapped in a `Promise`. Lifecycle internals (`constructor`, `free`, `Symbol.dispose`) are excluded on the type level and blocked at runtime by the worker's `isExposedMethodName` check; the proxy instead offers `dispose(): Promise<void>` which frees the WASM client and terminates the worker.

`MuxsocialClientWasm.create_new()` is `async` from day one so the RPC shape never changes when real (asynchronous) initialisation logic arrives.
