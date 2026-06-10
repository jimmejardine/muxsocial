import { Button, Group, Modal, Text } from "@mantine/core";
import { useTranslation } from "react-i18next";

interface ConfirmModalProps {
	opened: boolean;
	title: string;
	message: string;
	/** Label for the destructive confirm button (e.g. "Remove"). */
	confirmLabel: string;
	onConfirm: () => void;
	onClose: () => void;
}

/**
 * A small confirm dialog for destructive actions: a message and a Cancel /
 * destructive-confirm button pair. Mantine's `Modal` provides the focus trap and
 * Escape / backdrop close (both routed to `onClose`).
 */
export function ConfirmModal({ opened, title, message, confirmLabel, onConfirm, onClose }: ConfirmModalProps) {
	const { t } = useTranslation();

	return (
		<Modal opened={opened} onClose={onClose} title={title} size="sm" centered>
			<Text size="sm">{message}</Text>
			<Group justify="flex-end" gap="xs" mt="md">
				<Button variant="default" onClick={onClose}>
					{t("common.cancel")}
				</Button>
				<Button color="red" onClick={onConfirm}>
					{confirmLabel}
				</Button>
			</Group>
		</Modal>
	);
}
