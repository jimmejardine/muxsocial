import { Burger, Button, Menu, Popover, Stack } from "@mantine/core";
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
	/** Open the "Getting started" help wizard. */
	on_open_help: () => void;
	/** Open the config import/export dialog. */
	on_open_config: () => void;
	/** Open the "My accounts" dialog. */
	on_open_accounts: () => void;
	/** Open the nostr relays settings dialog. */
	on_open_relays: () => void;
}

/**
 * A hamburger that tucks the help wizard, version link, networks, theme and language
 * switchers away to the right of the toolbar's primary actions. The switchers keep
 * their own dropdowns, rendered in-place (not portalled) so opening one does not
 * dismiss this popover.
 */
export function HeaderMenu({ version, on_open_help, on_open_config, on_open_accounts, on_open_relays }: HeaderMenuProps) {
	const { t } = useTranslation();
	const [opened, handlers] = useDisclosure(false);

	return (
		<Popover opened={opened} onChange={(isOpen) => (isOpen ? handlers.open() : handlers.close())} position="bottom-end" withinPortal>
			<Popover.Target>
				<Burger opened={opened} onClick={handlers.toggle} size="sm" aria-label={t("toolbar.menu_label")} />
			</Popover.Target>
			<Popover.Dropdown p="xs">
				<Stack gap="xs">
					<Button
						size="xs"
						variant="default"
						onClick={() => {
							handlers.close();
							on_open_help();
						}}
					>
						{t("wizard.open")}
					</Button>
					<Menu position="bottom-end" withinPortal={false}>
						<Menu.Target>
							<Button size="xs" variant="default">
								{t("posting.menu")}
							</Button>
						</Menu.Target>
						<Menu.Dropdown>
							<Menu.Label>{t("posting.menu")}</Menu.Label>
							<Menu.Item
								onClick={() => {
									handlers.close();
									on_open_accounts();
								}}
							>
								{t("accounts.open")}
							</Menu.Item>
							<Menu.Item
								onClick={() => {
									handlers.close();
									on_open_relays();
								}}
							>
								{t("relays.open")}
							</Menu.Item>
						</Menu.Dropdown>
					</Menu>
					<LanguageSwitcher />
					<ThemeSwitcher />
					<NetworksMenu />
					<Button
						size="xs"
						variant="default"
						onClick={() => {
							handlers.close();
							on_open_config();
						}}
					>
						{t("config.open")}
					</Button>
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
