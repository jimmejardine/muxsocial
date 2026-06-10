import { AppShell } from "@mantine/core";
import { type CSSProperties, useCallback, useState } from "react";
import { StatusBar } from "./components/StatusBar.tsx";
import type { TimelineDescriptor } from "./components/Timeline.tsx";
import { TimelineArea } from "./components/TimelineArea.tsx";
import { Toolbar } from "./components/Toolbar.tsx";

const HEADER_HEIGHT = 52;
const FOOTER_HEIGHT = 32;

function create_timeline(): TimelineDescriptor {
	return { id: crypto.randomUUID() };
}

/** The app shell: toolbar header, statusbar footer, and side-by-side timelines. */
export function App() {
	const [timelines, set_timelines] = useState<TimelineDescriptor[]>(() => [create_timeline()]);

	const add_timeline = useCallback(() => {
		set_timelines((current) => [...current, create_timeline()]);
	}, []);

	const remove_timeline = useCallback((id: string) => {
		set_timelines((current) => current.filter((timeline) => timeline.id !== id));
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
				<TimelineArea timelines={timelines} on_remove={remove_timeline} />
			</AppShell.Main>

			<AppShell.Footer>
				<StatusBar timeline_count={timelines.length} />
			</AppShell.Footer>
		</AppShell>
	);
}
