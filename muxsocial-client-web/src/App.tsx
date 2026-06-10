import { AppShell, Center, Loader } from "@mantine/core";
import { type CSSProperties, useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { StatusBar } from "./components/StatusBar.tsx";
import { TimelineArea } from "./components/TimelineArea.tsx";
import { Toolbar } from "./components/Toolbar.tsx";
import { Muxsocial, type MuxsocialClientWasmProxy } from "./Muxsocial.ts";
import { MuxsocialContext } from "./tools/MuxsocialContext.tsx";
import type { TimelineConfig } from "./tools/TimelineConfig.ts";
import { Toast } from "./tools/Toast.ts";

/** The error's message, for interpolation into a toast string. */
function error_message(err: unknown): string {
	return err instanceof Error ? err.message : String(err);
}

const HEADER_HEIGHT = 52;
const FOOTER_HEIGHT = 32;

/**
 * The app shell. All timeline state lives in the Rust layer; this component is a
 * view: it seeds from `list_timelines()` and replaces its state with the snapshot
 * each command (`add_timeline` / `remove_timeline` / `add_source_to_timeline`)
 * returns. Every timeline is addressed by its Rust-minted GUID.
 */
export function App() {
	const { t } = useTranslation();
	const muxsocial_ref = useRef<MuxsocialClientWasmProxy | null>(null);
	// null while the Rust client is being created and the list loaded.
	const [timelines, set_timelines] = useState<TimelineConfig[] | null>(null);
	// Provided via context so per-timeline post lists can call the client.
	const [muxsocial_client, set_muxsocial_client] = useState<MuxsocialClientWasmProxy | null>(null);
	const [app_version, set_app_version] = useState<string | null>(null);

	useEffect(() => {
		let cancelled = false;
		(async () => {
			try {
				const muxsocial = await Muxsocial.create();
				muxsocial_ref.current = muxsocial;
				const initial_timelines = (await muxsocial.list_timelines()) as TimelineConfig[];
				const version = await muxsocial.version();
				if (!cancelled) {
					set_muxsocial_client(muxsocial);
					set_timelines(initial_timelines);
					set_app_version(version);
				}
			} catch (err) {
				if (!cancelled) {
					Toast.error(t("toast.error_load", { message: error_message(err) }));
				}
			}
		})();
		return () => {
			cancelled = true;
		};
	}, [t]);

	const add_timeline = useCallback(async () => {
		const muxsocial = muxsocial_ref.current;
		if (!muxsocial) return;
		try {
			set_timelines((await muxsocial.add_timeline()) as TimelineConfig[]);
			Toast.success(t("toast.timeline_added"));
		} catch (err) {
			Toast.error(t("toast.error_add_timeline", { message: error_message(err) }));
		}
	}, [t]);

	const remove_timeline = useCallback(
		async (id: string) => {
			const muxsocial = muxsocial_ref.current;
			if (!muxsocial) return;
			try {
				set_timelines((await muxsocial.remove_timeline(id)) as TimelineConfig[]);
				Toast.success(t("toast.timeline_removed"));
			} catch (err) {
				Toast.error(t("toast.error_remove_timeline", { message: error_message(err) }));
			}
		},
		[t],
	);

	const add_source = useCallback(
		async (id: string, address: string) => {
			const muxsocial = muxsocial_ref.current;
			if (!muxsocial) return;
			try {
				set_timelines((await muxsocial.add_source_to_timeline(id, address)) as TimelineConfig[]);
				Toast.success(t("toast.source_added"));
			} catch (err) {
				Toast.error(t("toast.error_add_source", { message: error_message(err) }));
			}
		},
		[t],
	);

	const set_name = useCallback(
		async (id: string, name: string) => {
			const muxsocial = muxsocial_ref.current;
			if (!muxsocial) return;
			try {
				set_timelines((await muxsocial.set_timeline_name(id, name)) as TimelineConfig[]);
			} catch (err) {
				Toast.error(t("toast.error_set_name", { message: error_message(err) }));
			}
		},
		[t],
	);

	// The timeline area's height is calc(100dvh - header - footer); expose the
	// header/footer heights as CSS vars so that calc stays in sync with AppShell.
	const content_height_vars = {
		"--mux-header-height": `${HEADER_HEIGHT}px`,
		"--mux-footer-height": `${FOOTER_HEIGHT}px`,
	} as CSSProperties;

	return (
		<MuxsocialContext.Provider value={muxsocial_client}>
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
						<TimelineArea timelines={timelines} on_remove={remove_timeline} on_add_source={add_source} on_set_name={set_name} />
					)}
				</AppShell.Main>

				<AppShell.Footer>
					<StatusBar timeline_count={timelines?.length ?? 0} version={app_version} />
				</AppShell.Footer>
			</AppShell>
		</MuxsocialContext.Provider>
	);
}
