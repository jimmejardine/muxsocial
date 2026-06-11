import { Button, Group, Modal, TextInput } from "@mantine/core";
import { useState } from "react";
import { useTranslation } from "react-i18next";

interface AddSourceModalProps {
	opened: boolean;
	onClose: () => void;
	onSubmit: (address: string) => void;
}

/**
 * A small dialog for adding a source: a single "paste an address" input with an
 * OK / Cancel pair. It owns the draft value and clears it on close so each open
 * starts empty. Mantine's `Modal` provides the focus trap and Escape / backdrop
 * close (both routed to `close`); `data-autofocus` focuses the input on open.
 */
export function AddSourceModal({ opened, onClose, onSubmit }: AddSourceModalProps) {
	const { t } = useTranslation();
	const [address, set_address] = useState("");

	const close = () => {
		set_address("");
		onClose();
	};

	const submit = () => {
		const trimmed_address = address.trim();
		if (trimmed_address.length > 0) {
			onSubmit(trimmed_address);
		}
		close();
	};

	return (
		<Modal opened={opened} onClose={close} title={t("timeline.add_source")} size="sm" centered>
			<form
				onSubmit={(event) => {
					event.preventDefault();
					submit();
				}}
			>
				<TextInput data-autofocus size="sm" placeholder={t("timeline.address_placeholder")} value={address} onChange={(event) => set_address(event.currentTarget.value)} enterKeyHint="go" />
				<Group justify="flex-end" gap="xs" mt="md">
					<Button variant="default" onClick={close}>
						{t("common.cancel")}
					</Button>
					<Button type="submit" disabled={address.trim().length === 0}>
						{t("common.ok")}
					</Button>
				</Group>
			</form>
		</Modal>
	);
}
