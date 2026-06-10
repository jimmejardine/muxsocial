/**
 * Exposes the {@link MuxsocialClientWasmProxy} to descendants so per-timeline
 * components (e.g. the post list) can call the WASM client without prop-drilling.
 * The value is `null` until the worker-backed client has been created.
 *
 * @module
 */

import { createContext, useContext } from "react";
import type { MuxsocialClientWasmProxy } from "../Muxsocial.ts";

export const MuxsocialContext = createContext<MuxsocialClientWasmProxy | null>(null);

/** The muxsocial client proxy, or `null` while it is still being created. */
export function useMuxsocial(): MuxsocialClientWasmProxy | null {
	return useContext(MuxsocialContext);
}
