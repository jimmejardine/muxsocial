import { Burger, Popover, Stack } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { useTranslation } from "react-i18next";
import { LanguageSwitcher } from "./LanguageSwitcher.tsx";
import { NetworksMenu } from "./NetworksMenu.tsx";
import { ThemeSwitcher } from "./ThemeSwitcher.tsx";

/**
 * A hamburger that tucks the theme and language switchers away to the right of
 * the toolbar's primary actions. The switchers keep their own dropdowns, rendered
 * in-place (not portalled) so opening one does not dismiss this popover.
 */
export function HeaderMenu() {
	const { t } = useTranslation();
	const [opened, handlers] = useDisclosure(false);

	return (
		<Popover opened={opened} onChange={(isOpen) => (isOpen ? handlers.open() : handlers.close())} position="bottom-end" withinPortal>
			<Popover.Target>
				<Burger opened={opened} onClick={handlers.toggle} size="sm" aria-label={t("toolbar.menu_label")} />
			</Popover.Target>
			<Popover.Dropdown p="xs">
				<Stack gap="xs">
					<NetworksMenu />
					<ThemeSwitcher />
					<LanguageSwitcher />
				</Stack>
			</Popover.Dropdown>
		</Popover>
	);
}
