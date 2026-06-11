import { Button, Menu } from "@mantine/core";
import { useTranslation } from "react-i18next";
import { useMuxTheme } from "../theme/MuxThemeProvider.tsx";

/** A menu that switches between the registered mux themes. */
export function ThemeSwitcher() {
	const { t } = useTranslation();
	const { theme, themeId, setThemeId, themes } = useMuxTheme();

	return (
		<Menu position="bottom-end" withinPortal={false}>
			<Menu.Target>
				<Button size="xs" variant="default">
					{theme.label}
				</Button>
			</Menu.Target>
			<Menu.Dropdown>
				<Menu.Label>{t("theme.label")}</Menu.Label>
				{themes.map((candidate) => (
					<Menu.Item key={candidate.id} onClick={() => setThemeId(candidate.id)} fw={candidate.id === themeId ? 700 : undefined}>
						{candidate.label}
					</Menu.Item>
				))}
			</Menu.Dropdown>
		</Menu>
	);
}
