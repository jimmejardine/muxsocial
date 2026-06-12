import { Button, CloseButton, Group, Modal, Text } from "@mantine/core";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import classes from "./HelpWizard.module.css";

interface WizardPage {
	/** Image shown above the text. Same dimensions as the welcome image (1024×512). */
	image: string;
	/** i18n key for the page's body text. */
	textKey: string;
}

// Page 1 reuses the existing welcome banner; pages 2–4 have their own art (same
// 1024×512 dimensions) under /img/wizard/.
const WELCOME_IMAGE = "/img/muxsocial.jpg";
const WIZARD_PAGES: WizardPage[] = [
	{ image: WELCOME_IMAGE, textKey: "wizard.page1" },
	{ image: "/img/wizard/timelines.jpg", textKey: "wizard.page2" },
	{ image: "/img/wizard/add-source.jpg", textKey: "wizard.page3" },
	{ image: "/img/wizard/done.jpg", textKey: "wizard.page4" },
];

/** The paged wizard body: an image above text, step dots, and Back / Next (the last
 *  page's button just closes the wizard — adding the first timeline is left to the
 *  user, guided by the pulsing toolbar button). */
function HelpWizardContent({ onDone }: { onDone: () => void }) {
	const { t } = useTranslation();
	const [page, setPage] = useState(0);
	const is_first = page === 0;
	const is_last = page === WIZARD_PAGES.length - 1;
	const current = WIZARD_PAGES[page];

	return (
		<div className={classes.content}>
			<img className={classes.image} src={current.image} alt="" />
			<Text className={classes.text}>{t(current.textKey)}</Text>

			<div className={classes.dots} aria-hidden="true">
				{WIZARD_PAGES.map((wizard_page, index) => (
					<span key={wizard_page.textKey} className={index === page ? classes.dotActive : classes.dot} />
				))}
			</div>

			<Group justify="space-between" mt="md" wrap="nowrap">
				<Button variant="default" onClick={() => setPage((previous) => previous - 1)} disabled={is_first}>
					{t("wizard.back")}
				</Button>
				{is_last ? <Button onClick={onDone}>{t("wizard.done")}</Button> : <Button onClick={() => setPage((previous) => previous + 1)}>{t("wizard.next")}</Button>}
			</Group>
		</div>
	);
}

interface HelpWizardProps {
	opened: boolean;
	onClose: () => void;
}

/**
 * The onboarding "Getting started" wizard as a modal overlay. Opened from the
 * header menu and auto-opened once when the app first loads with no timelines.
 * Keyed on `opened` so it resets to the first page each time it is shown.
 */
export function HelpWizard({ opened, onClose }: HelpWizardProps) {
	const { t } = useTranslation();
	// Drop Mantine's header (it reserves a row even with just an X) and overlay our
	// own close button in the body's top-right corner, so the title bar takes no space.
	return (
		<Modal opened={opened} onClose={onClose} withCloseButton={false} size="lg" centered styles={{ body: { position: "relative" } }}>
			<CloseButton
				onClick={onClose}
				aria-label={t("wizard.close")}
				style={{ position: "absolute", top: "calc(var(--mantine-spacing-md) * 1.5)", right: "calc(var(--mantine-spacing-md) * 1.5)", zIndex: 2 }}
			/>
			<HelpWizardContent key={opened ? "open" : "closed"} onDone={onClose} />
		</Modal>
	);
}
