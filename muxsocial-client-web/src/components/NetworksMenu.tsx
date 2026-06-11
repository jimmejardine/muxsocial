import { Button, Menu } from "@mantine/core";
import { useTranslation } from "react-i18next";
import { NETWORK_LINKS } from "../tools/networks.ts";

/** A menu listing the source networks; each item opens that network's homepage. */
export function NetworksMenu() {
	const { t } = useTranslation();

	return (
		<Menu position="bottom-end" withinPortal={false}>
			<Menu.Target>
				<Button size="xs" variant="default">
					{t("toolbar.networks")}
				</Button>
			</Menu.Target>
			<Menu.Dropdown>
				<Menu.Label>{t("toolbar.networks")}</Menu.Label>
				{NETWORK_LINKS.map((network) => (
					<Menu.Item key={network.name} component="a" href={network.url} target="_blank" rel="noreferrer">
						{network.name}
					</Menu.Item>
				))}
			</Menu.Dropdown>
		</Menu>
	);
}
