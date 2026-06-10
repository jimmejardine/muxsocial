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

The worker processes requests **serially** — each `onmessage` is chained onto a promise so one request fully completes before the next begins. This keeps the WASM client's `&mut self` command methods (e.g. `add_timeline`) from re-entering, which would panic with "recursive use of an object".

## State sync: command → snapshot (pull, not push)

All app state lives in Rust; the GUI is a view. There is **no async push** from Rust to the GUI. Instead:

- The GUI seeds itself with a query (`list_timelines()`).
- Every mutation is a GUID-addressed command (`add_timeline()`, `remove_timeline(id)`, `add_source_to_timeline(id, address)`) that mutates Rust state, **serialises it to the `ConfigStorage` created at startup**, and **returns the new full snapshot**. The GUI replaces its render state with that snapshot — so the command reply *is* the change notification.

The Rust `TimelineRegistry` (`muxsocial-lib/src/timeline_registry.rs`) owns the timeline list; snapshots cross the boundary as `serde_wasm_bindgen` values (`TimelineConfig[]`). If state ever changes *without* a GUI action (e.g. live post streams), that is handled by per-component pull/poll, not a push channel.

## Typing

`MuxsocialClientWasmProxy` is derived from the wasm-bindgen-generated `MuxsocialClientWasm` type: every public method is mirrored with its return type wrapped in a `Promise`. Lifecycle internals (`constructor`, `free`, `Symbol.dispose`) are excluded on the type level and blocked at runtime by the worker's `isExposedMethodName` check; the proxy instead offers `dispose(): Promise<void>` which frees the WASM client and terminates the worker.

`MuxsocialClientWasm.create_new()` is `async` from day one so the RPC shape never changes when real (asynchronous) initialisation logic arrives.
