import { Button, Menu } from "@mantine/core";
import { useMuxTheme } from "../theme/MuxThemeProvider.tsx";

/** A menu that switches between the registered mux themes. */
export function ThemeSwitcher() {
	const { theme, themeId, setThemeId, themes } = useMuxTheme();

	return (
		<Menu position="bottom-end" withinPortal>
			<Menu.Target>
				<Button size="xs" variant="default">
					{theme.label}
				</Button>
			</Menu.Target>
			<Menu.Dropdown>
				<Menu.Label>Theme</Menu.Label>
				{themes.map((candidate) => (
					<Menu.Item key={candidate.id} onClick={() => setThemeId(candidate.id)} fw={candidate.id === themeId ? 700 : undefined}>
						{candidate.label}
					</Menu.Item>
				))}
			</Menu.Dropdown>
		</Menu>
	);
}
