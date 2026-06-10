import { Anchor, Group, Text } from "@mantine/core";
import { useTranslation } from "react-i18next";

const RELEASES_URL = "https://github.com/jimmejardine/muxsocial/releases";

interface StatusBarProps {
	timeline_count: number;
	/** The app version (from the Rust Cargo.toml), or null until loaded. */
	version: string | null;
}

/** The bottom statusbar: timeline count on the left, the app version (a link to
 *  the releases page) on the right. */
export function StatusBar({ timeline_count, version }: StatusBarProps) {
	const { t } = useTranslation();

	return (
		<Group h="100%" px="sm" justify="space-between" wrap="nowrap">
			<Text size="xs" c="dimmed">
				{t("status.timelines", { count: timeline_count })}
			</Text>
			{version && (
				<Anchor href={RELEASES_URL} target="_blank" rel="noreferrer" size="xs" c="dimmed" title={t("status.releases")}>
					v{version}
				</Anchor>
			)}
		</Group>
	);
}
