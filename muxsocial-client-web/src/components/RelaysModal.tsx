import { Button, Group, Modal, Stack, Text, Textarea } from "@mantine/core";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useMuxsocial } from "../tools/MuxsocialContext.tsx";
import { Toast } from "../tools/Toast.ts";

interface RelaysModalProps {
	opened: boolean;
	onClose: () => void;
}

function message_of(error: unknown): string {
	return error instanceof Error ? error.message : String(error);
}

/**
 * Edit the nostr relay list as a single `;`-separated string (it governs both
 * reading and posting). Loads the current effective list on open, saves via
 * `set_nostr_relays`, and applies at runtime (the worker reconnects). Mirrors the
 * small single-input dialogs (e.g. AddSourceModal).
 */
export function RelaysModal({ opened, onClose }: RelaysModalProps) {
	const { t } = useTranslation();
	const muxsocial = useMuxsocial();

	const [relays_text, set_relays_text] = useState("");
	const [saving, set_saving] = useState(false);

	// Load the current relay string each time the dialog opens.
	useEffect(() => {
		if (!opened || !muxsocial) return;
		let cancelled = false;
		(async () => {
			try {
				const current = await muxsocial.get_nostr_relays();
				if (!cancelled) set_relays_text(current);
			} catch (caught) {
				if (!cancelled) Toast.error(t("toast.error_relays_save", { message: message_of(caught) }));
			}
		})();
		return () => {
			cancelled = true;
		};
	}, [opened, muxsocial, t]);

	const save = async () => {
		if (!muxsocial) return;
		set_saving(true);
		try {
			const normalized = await muxsocial.set_nostr_relays(relays_text);
			set_relays_text(normalized);
			Toast.success(t("toast.relays_saved"));
			onClose();
		} catch (caught) {
			Toast.error(t("toast.error_relays_save", { message: message_of(caught) }));
		} finally {
			set_saving(false);
		}
	};

	return (
		<Modal opened={opened} onClose={onClose} title={t("relays.title")} size="md" centered>
			<Stack gap="sm">
				<Text size="sm" c="dimmed">
					{t("relays.help")}
				</Text>
				<Textarea data-autofocus autosize minRows={2} maxRows={8} placeholder={t("relays.placeholder")} value={relays_text} onChange={(event) => set_relays_text(event.currentTarget.value)} />
				<Group justify="flex-end" gap="xs">
					<Button variant="default" onClick={onClose}>
						{t("common.cancel")}
					</Button>
					<Button onClick={save} loading={saving}>
						{t("common.save")}
					</Button>
				</Group>
			</Stack>
		</Modal>
	);
}
