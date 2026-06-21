import { Anchor, Button, Group, Text, Title } from "@mantine/core";
import { Trans, useTranslation } from "react-i18next";
import { NETWORK_LINKS } from "../tools/networks.ts";
import { HeaderMenu } from "./HeaderMenu.tsx";

interface ToolbarProps {
	on_add_timeline: () => void;
	/** Open the compose / cross-post dialog. */
	on_open_compose: () => void;
	/** Pulse the Add-timeline button for attention (no timelines yet). */
	highlight?: boolean;
	/** App version, shown in the hamburger menu; null until loaded. */
	version?: string | null;
	/** Open the "Getting started" help wizard (from the hamburger menu). */
	on_open_help: () => void;
	/** Open the config import/export dialog (from the hamburger menu). */
	on_open_config: () => void;
	/** Open the "My accounts" dialog (from the hamburger menu). */
	on_open_accounts: () => void;
	/** Open the nostr relays settings dialog (from the hamburger menu). */
	on_open_relays: () => void;
}

/** A simple quill/pen glyph for the Post button. */
function PenIcon() {
	return (
		<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
			<path d="M12 20h9" />
			<path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4Z" />
		</svg>
	);
}

/** The top toolbar: app title, theme/language switchers, and the Post / Add-timeline buttons. */
export function Toolbar({ on_add_timeline, on_open_compose, highlight, version, on_open_help, on_open_config, on_open_accounts, on_open_relays }: ToolbarProps) {
	const { t } = useTranslation();

	return (
		<Group h="100%" px="sm" justify="space-between" wrap="nowrap" gap="sm">
			<Group gap="xs" wrap="nowrap" style={{ flex: 1, minWidth: 0 }}>
				<img src="/img/favicon/favicon.png" alt="" width={28} height={28} style={{ display: "block", borderRadius: 4, flexShrink: 0 }} />
				<Title order={4} style={{ flexShrink: 0 }}>
					mux.social
				</Title>
				<Text size="sm" c="dimmed" truncate style={{ flex: 1, minWidth: 0 }}>
					<Trans i18nKey="toolbar.tagline" components={NETWORK_LINKS.map((network) => <Anchor key={network.name} href={network.url} target="_blank" rel="noreferrer" inherit />)} />
				</Text>
			</Group>
			<Group gap="xs" wrap="nowrap" style={{ flexShrink: 0 }}>
				<Button size="xs" variant="default" leftSection={<PenIcon />} onClick={on_open_compose}>
					{t("compose.open")}
				</Button>
				<Button size="xs" onClick={on_add_timeline} className={highlight ? "mux-throb" : undefined}>
					{t("toolbar.add_timeline")}
				</Button>
				<HeaderMenu version={version} on_open_help={on_open_help} on_open_config={on_open_config} on_open_accounts={on_open_accounts} on_open_relays={on_open_relays} />
			</Group>
		</Group>
	);
}
