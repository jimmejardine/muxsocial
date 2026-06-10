import { Button, Menu } from "@mantine/core";
import { useTranslation } from "react-i18next";
import { SUPPORTED_LANGUAGES, set_language } from "../i18n/i18n.ts";

/** A menu that switches the UI language between the supported locales. */
export function LanguageSwitcher() {
	const { t, i18n } = useTranslation();

	const current_language = SUPPORTED_LANGUAGES.find((language) => language.value === i18n.language) ?? SUPPORTED_LANGUAGES[0];

	return (
		<Menu position="bottom-end" withinPortal>
			<Menu.Target>
				<Button size="xs" variant="default">
					{current_language.label}
				</Button>
			</Menu.Target>
			<Menu.Dropdown>
				<Menu.Label>{t("language.label")}</Menu.Label>
				{SUPPORTED_LANGUAGES.map((language) => (
					<Menu.Item key={language.value} onClick={() => set_language(language.value)} fw={language.value === i18n.language ? 700 : undefined}>
						{language.label}
					</Menu.Item>
				))}
			</Menu.Dropdown>
		</Menu>
	);
}
