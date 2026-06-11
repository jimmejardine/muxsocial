import { Burger, Button, Popover, Stack } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { useTranslation } from "react-i18next";
import { LanguageSwitcher } from "./LanguageSwitcher.tsx";
import { NetworksMenu } from "./NetworksMenu.tsx";
import { ThemeSwitcher } from "./ThemeSwitcher.tsx";

const GITHUB_URL = "https://github.com/jimmejardine/muxsocial";
const RELEASES_URL = "https://github.com/jimmejardine/muxsocial/releases";

interface HeaderMenuProps {
	/** App version shown as the first menu item (links to releases); null until loaded. */
	version?: string | null;
}

/**
 * A hamburger that tucks the version link, networks, theme and language switchers
 * away to the right of the toolbar's primary actions. The switchers keep their own
 * dropdowns, rendered in-place (not portalled) so opening one does not dismiss
 * this popover.
 */
export function HeaderMenu({ version }: HeaderMenuProps) {
	const { t } = useTranslation();
	const [opened, handlers] = useDisclosure(false);

	return (
		<Popover opened={opened} onChange={(isOpen) => (isOpen ? handlers.open() : handlers.close())} position="bottom-end" withinPortal>
			<Popover.Target>
				<Burger opened={opened} onClick={handlers.toggle} size="sm" aria-label={t("toolbar.menu_label")} />
			</Popover.Target>
			<Popover.Dropdown p="xs">
				<Stack gap="xs">
					<LanguageSwitcher />
					<ThemeSwitcher />
					<NetworksMenu />
					<Button component="a" href={GITHUB_URL} target="_blank" rel="noreferrer" size="xs" variant="default" title={t("status.github")}>
						Github
					</Button>
					{version && (
						<Button component="a" href={RELEASES_URL} target="_blank" rel="noreferrer" size="xs" variant="default" title={t("status.releases")}>
							v{version}
						</Button>
					)}
				</Stack>
			</Popover.Dropdown>
		</Popover>
	);
}
