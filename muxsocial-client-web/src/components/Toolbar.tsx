import { Button, Group, Title } from "@mantine/core";
import { useTranslation } from "react-i18next";
import { LanguageSwitcher } from "./LanguageSwitcher.tsx";
import { ThemeSwitcher } from "./ThemeSwitcher.tsx";

interface ToolbarProps {
	on_add_timeline: () => void;
}

/** The top toolbar: app title, theme/language switchers, and the "Add timeline" button. */
export function Toolbar({ on_add_timeline }: ToolbarProps) {
	const { t } = useTranslation();

	return (
		<Group h="100%" px="sm" justify="space-between" wrap="nowrap">
			<Title order={4}>mux.social</Title>
			<Group gap="xs" wrap="nowrap">
				<LanguageSwitcher />
				<ThemeSwitcher />
				<Button size="xs" onClick={on_add_timeline}>
					{t("toolbar.add_timeline")}
				</Button>
			</Group>
		</Group>
	);
}
