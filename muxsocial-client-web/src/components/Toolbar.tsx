import { Button, Group, Title } from "@mantine/core";
import { ThemeSwitcher } from "./ThemeSwitcher.tsx";

interface ToolbarProps {
	on_add_timeline: () => void;
}

/** The top toolbar: app title, theme switcher, and the "Add timeline" button. */
export function Toolbar({ on_add_timeline }: ToolbarProps) {
	return (
		<Group h="100%" px="sm" justify="space-between" wrap="nowrap">
			<Title order={4}>mux.social</Title>
			<Group gap="xs" wrap="nowrap">
				<ThemeSwitcher />
				<Button size="xs" onClick={on_add_timeline}>
					Add timeline
				</Button>
			</Group>
		</Group>
	);
}
