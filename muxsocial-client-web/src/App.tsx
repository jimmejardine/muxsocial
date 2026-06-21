import { AppShell, Center, Loader } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { type CSSProperties, useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { AccountsModal } from "./components/AccountsModal.tsx";
import { ComposeModal } from "./components/ComposeModal.tsx";
import { ConfigDialog } from "./components/ConfigDialog.tsx";
import { HelpWizard } from "./components/HelpWizard.tsx";
import { RelaysModal } from "./components/RelaysModal.tsx";
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
	const [help_opened, help_handlers] = useDisclosure(false);
	const [config_opened, config_handlers] = useDisclosure(false);
	const [compose_opened, compose_handlers] = useDisclosure(false);
	const [accounts_opened, accounts_handlers] = useDisclosure(false);
	const [relays_opened, relays_handlers] = useDisclosure(false);
	// Bumped on account add/remove so an open ComposeModal refreshes its list.
	const [accounts_version, set_accounts_version] = useState(0);
	// Auto-open the getting-started wizard once when the app first loads with no timelines.
	const has_auto_opened_help = useRef(false);

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

	useEffect(() => {
		if (timelines !== null && timelines.length === 0 && !has_auto_opened_help.current) {
			has_auto_opened_help.current = true;
			help_handlers.open();
		}
	}, [timelines, help_handlers]);

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

	const remove_source = useCallback(
		async (id: string, network: string, source_id: string) => {
			const muxsocial = muxsocial_ref.current;
			if (!muxsocial) return;
			try {
				set_timelines((await muxsocial.remove_source_from_timeline(id, network, source_id)) as TimelineConfig[]);
				Toast.success(t("toast.source_removed"));
			} catch (err) {
				Toast.error(t("toast.error_remove_source", { message: error_message(err) }));
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

	const set_autopoll = useCallback(
		async (id: string, autopoll: boolean) => {
			const muxsocial = muxsocial_ref.current;
			if (!muxsocial) return;
			try {
				set_timelines((await muxsocial.set_timeline_autopoll(id, autopoll)) as TimelineConfig[]);
				Toast.success(t(autopoll ? "toast.autorefresh_on" : "toast.autorefresh_off"));
			} catch (err) {
				Toast.error(t("toast.error_set_autopoll", { message: error_message(err) }));
			}
		},
		[t],
	);

	// The timeline area's height is calc(100dvh - header - footer); expose the
	// header height as a CSS var so that calc stays in sync with AppShell. There is
	// no footer anymore, so the footer var is zero.
	const content_height_vars = {
		"--mux-header-height": `${HEADER_HEIGHT}px`,
		"--mux-footer-height": "0px",
	} as CSSProperties;

	return (
		<MuxsocialContext.Provider value={muxsocial_client}>
			<AppShell header={{ height: HEADER_HEIGHT }} padding={0} style={content_height_vars}>
				<AppShell.Header>
					<Toolbar
						on_add_timeline={add_timeline}
						on_open_compose={compose_handlers.open}
						highlight={timelines !== null && timelines.length === 0}
						version={app_version}
						on_open_help={help_handlers.open}
						on_open_config={config_handlers.open}
						on_open_accounts={accounts_handlers.open}
						on_open_relays={relays_handlers.open}
					/>
				</AppShell.Header>

				<AppShell.Main>
					{timelines === null ? (
						<Center h="100%">
							<Loader />
						</Center>
					) : (
						<TimelineArea
							timelines={timelines}
							on_remove={remove_timeline}
							on_add_source={add_source}
							on_remove_source={remove_source}
							on_set_name={set_name}
							on_set_autopoll={set_autopoll}
							on_open_help={help_handlers.open}
						/>
					)}
				</AppShell.Main>
			</AppShell>
			<HelpWizard opened={help_opened} onClose={help_handlers.close} />
			<ConfigDialog opened={config_opened} onClose={config_handlers.close} on_timelines_imported={set_timelines} />
			<ComposeModal opened={compose_opened} onClose={compose_handlers.close} onOpenAccounts={accounts_handlers.open} accountsVersion={accounts_version} />
			<AccountsModal opened={accounts_opened} onClose={accounts_handlers.close} onAccountsChanged={() => set_accounts_version((version) => version + 1)} />
			<RelaysModal opened={relays_opened} onClose={relays_handlers.close} />
		</MuxsocialContext.Provider>
	);
}
