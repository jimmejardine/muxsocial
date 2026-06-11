import { Button, Group, Title } from "@mantine/core";
import { useTranslation } from "react-i18next";
import { LanguageSwitcher } from "./LanguageSwitcher.tsx";
import { ThemeSwitcher } from "./ThemeSwitcher.tsx";

interface ToolbarProps {
	on_add_timeline: () => void;
	/** Pulse the Add-timeline button for attention (no timelines yet). */
	highlight?: boolean;
}

/** The top toolbar: app title, theme/language switchers, and the "Add timeline" button. */
export function Toolbar({ on_add_timeline, highlight }: ToolbarProps) {
	const { t } = useTranslation();

	return (
		<Group h="100%" px="sm" justify="space-between" wrap="nowrap">
			<Group gap="xs" wrap="nowrap">
				<img src="/img/favicon/favicon.png" alt="" width={28} height={28} style={{ display: "block", borderRadius: 4 }} />
				<Title order={4}>mux.social</Title>
			</Group>
			<Group gap="xs" wrap="nowrap">
				<LanguageSwitcher />
				<ThemeSwitcher />
				<Button size="xs" onClick={on_add_timeline} className={highlight ? "mux-throb" : undefined}>
					{t("toolbar.add_timeline")}
				</Button>
			</Group>
		</Group>
	);
}
