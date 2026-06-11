import { Anchor, Button, Group, Text, Title } from "@mantine/core";
import { Trans, useTranslation } from "react-i18next";
import { NETWORK_LINKS } from "../tools/networks.ts";
import { HeaderMenu } from "./HeaderMenu.tsx";

interface ToolbarProps {
	on_add_timeline: () => void;
	/** Pulse the Add-timeline button for attention (no timelines yet). */
	highlight?: boolean;
	/** App version, shown in the hamburger menu; null until loaded. */
	version?: string | null;
}

/** The top toolbar: app title, theme/language switchers, and the "Add timeline" button. */
export function Toolbar({ on_add_timeline, highlight, version }: ToolbarProps) {
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
				<Button size="xs" onClick={on_add_timeline} className={highlight ? "mux-throb" : undefined}>
					{t("toolbar.add_timeline")}
				</Button>
				<HeaderMenu version={version} />
			</Group>
		</Group>
	);
}
