import { Group, Text } from "@mantine/core";

interface StatusBarProps {
	timeline_count: number;
}

/** The bottom statusbar. For now it just reports the timeline count. */
export function StatusBar({ timeline_count }: StatusBarProps) {
	return (
		<Group h="100%" px="sm" justify="flex-start">
			<Text size="xs" c="dimmed">
				{timeline_count} timeline{timeline_count === 1 ? "" : "s"}
			</Text>
		</Group>
	);
}
