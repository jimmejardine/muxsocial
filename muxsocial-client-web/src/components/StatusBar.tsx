import { Group, Text } from "@mantine/core";
import { useTranslation } from "react-i18next";

interface StatusBarProps {
	timeline_count: number;
}

/** The bottom statusbar. For now it just reports the timeline count. */
export function StatusBar({ timeline_count }: StatusBarProps) {
	const { t } = useTranslation();

	return (
		<Group h="100%" px="sm" justify="flex-start">
			<Text size="xs" c="dimmed">
				{t("status.timelines", { count: timeline_count })}
			</Text>
		</Group>
	);
}
