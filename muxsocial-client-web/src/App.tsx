import { AppShell, Center, Loader } from "@mantine/core";
import { type CSSProperties, useCallback, useEffect, useRef, useState } from "react";
import { StatusBar } from "./components/StatusBar.tsx";
import { TimelineArea } from "./components/TimelineArea.tsx";
import { Toolbar } from "./components/Toolbar.tsx";
import { Muxsocial, type MuxsocialClientWasmProxy } from "./Muxsocial.ts";
import type { TimelineConfig } from "./tools/TimelineConfig.ts";

const HEADER_HEIGHT = 52;
const FOOTER_HEIGHT = 32;

/**
 * The app shell. All timeline state lives in the Rust layer; this component is a
 * view: it seeds from `list_timelines()` and replaces its state with the snapshot
 * each command (`add_timeline` / `remove_timeline` / `add_source_to_timeline`)
 * returns. Every timeline is addressed by its Rust-minted GUID.
 */
export function App() {
	const muxsocial_ref = useRef<MuxsocialClientWasmProxy | null>(null);
	// null while the Rust client is being created and the list loaded.
	const [timelines, set_timelines] = useState<TimelineConfig[] | null>(null);

	useEffect(() => {
		let cancelled = false;
		(async () => {
			const muxsocial = await Muxsocial.create();
			muxsocial_ref.current = muxsocial;
			const initial_timelines = (await muxsocial.list_timelines()) as TimelineConfig[];
			if (!cancelled) {
				set_timelines(initial_timelines);
			}
		})();
		return () => {
			cancelled = true;
		};
	}, []);

	const add_timeline = useCallback(async () => {
		const muxsocial = muxsocial_ref.current;
		if (muxsocial) {
			set_timelines((await muxsocial.add_timeline()) as TimelineConfig[]);
		}
	}, []);

	const remove_timeline = useCallback(async (id: string) => {
		const muxsocial = muxsocial_ref.current;
		if (muxsocial) {
			set_timelines((await muxsocial.remove_timeline(id)) as TimelineConfig[]);
		}
	}, []);

	const add_source = useCallback(async (id: string, address: string) => {
		const muxsocial = muxsocial_ref.current;
		if (muxsocial) {
			set_timelines((await muxsocial.add_source_to_timeline(id, address)) as TimelineConfig[]);
		}
	}, []);

	// The timeline area's height is calc(100dvh - header - footer); expose the
	// header/footer heights as CSS vars so that calc stays in sync with AppShell.
	const content_height_vars = {
		"--mux-header-height": `${HEADER_HEIGHT}px`,
		"--mux-footer-height": `${FOOTER_HEIGHT}px`,
	} as CSSProperties;

	return (
		<AppShell header={{ height: HEADER_HEIGHT }} footer={{ height: FOOTER_HEIGHT }} padding={0} style={content_height_vars}>
			<AppShell.Header>
				<Toolbar on_add_timeline={add_timeline} />
			</AppShell.Header>

			<AppShell.Main>
				{timelines === null ? (
					<Center h="100%">
						<Loader />
					</Center>
				) : (
					<TimelineArea timelines={timelines} on_remove={remove_timeline} on_add_source={add_source} />
				)}
			</AppShell.Main>

			<AppShell.Footer>
				<StatusBar timeline_count={timelines?.length ?? 0} />
			</AppShell.Footer>
		</AppShell>
	);
}
