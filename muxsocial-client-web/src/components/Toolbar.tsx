import { Button, Group, Title } from "@mantine/core";

interface ToolbarProps {
	on_add_timeline: () => void;
}

/** The top toolbar: app title and the "Add timeline" button. */
export function Toolbar({ on_add_timeline }: ToolbarProps) {
	return (
		<Group h="100%" px="sm" justify="space-between" wrap="nowrap">
			<Title order={4}>mux.social</Title>
			<Button size="xs" onClick={on_add_timeline}>
				Add timeline
			</Button>
		</Group>
	);
}
