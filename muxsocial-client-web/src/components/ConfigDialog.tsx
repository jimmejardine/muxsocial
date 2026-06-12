import { Button, Group, Modal, Text, Textarea } from "@mantine/core";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { SUPPORTED_LANGUAGES, set_language } from "../i18n/i18n.ts";
import { useMuxTheme } from "../theme/MuxThemeProvider.tsx";
import { compose_config_text, parse_config_text } from "../tools/ConfigTransfer.ts";
import { useMuxsocial } from "../tools/MuxsocialContext.tsx";
import type { TimelineConfig } from "../tools/TimelineConfig.ts";
import { Toast } from "../tools/Toast.ts";

/** The error's message, for interpolation into a toast string. */
function error_message(err: unknown): string {
	return err instanceof Error ? err.message : String(err);
}

interface ConfigDialogProps {
	opened: boolean;
	onClose: () => void;
	/** Receives the new timeline snapshot after a successful Apply. */
	on_timelines_imported: (timelines: TimelineConfig[]) => void;
}

/**
 * The "Config" dialog: the whole configuration (timelines + GUI settings) as one
 * editable JSON document, for copy/paste transfer between machines. Apply
 * validates the text (the timelines deeply, all-or-nothing, in Rust; the settings
 * here) and applies it; Revert reloads the box from the current configuration.
 */
export function ConfigDialog({ opened, onClose, on_timelines_imported }: ConfigDialogProps) {
	const { t, i18n } = useTranslation();
	const muxsocial = useMuxsocial();
	const { themeId, setThemeId, themes } = useMuxTheme();
	const [config_text, set_config_text] = useState("");
	const [applying, set_applying] = useState(false);

	const load_current_config = useCallback(async () => {
		if (!muxsocial) return;
		try {
			const timelines_json = await muxsocial.export_timelines_json();
			set_config_text(compose_config_text(timelines_json, { theme: themeId, language: i18n.language }));
		} catch (err) {
			Toast.error(t("toast.error_config_load", { message: error_message(err) }));
		}
	}, [muxsocial, themeId, i18n.language, t]);

	// Seed the box from the current configuration each time the dialog opens.
	useEffect(() => {
		if (opened) {
			load_current_config();
		}
	}, [opened, load_current_config]);

	const apply = async () => {
		if (!muxsocial) return;
		set_applying(true);
		try {
			const parsed_config = parse_config_text(
				config_text,
				themes.map((candidate) => candidate.id),
				SUPPORTED_LANGUAGES.map((language) => language.value),
			);
			const imported_timelines = (await muxsocial.import_timelines_json(parsed_config.timelines_json)) as TimelineConfig[];
			if (parsed_config.settings.theme !== undefined) {
				setThemeId(parsed_config.settings.theme);
			}
			if (parsed_config.settings.language !== undefined) {
				set_language(parsed_config.settings.language);
			}
			on_timelines_imported(imported_timelines);
			Toast.success(t("toast.config_applied"));
			onClose();
		} catch (err) {
			Toast.error(t("toast.error_config_apply", { message: error_message(err) }));
		} finally {
			set_applying(false);
		}
	};

	return (
		<Modal opened={opened} onClose={onClose} title={t("config.title")} size="lg" centered>
			<Text size="sm" mb="sm">
				{t("config.description")}
			</Text>
			<Textarea
				value={config_text}
				onChange={(event) => set_config_text(event.currentTarget.value)}
				autosize
				minRows={12}
				maxRows={20}
				spellCheck={false}
				styles={{ input: { fontFamily: "var(--mantine-font-family-monospace)", fontSize: "var(--mantine-font-size-xs)" } }}
				aria-label={t("config.title")}
			/>
			<Group justify="flex-end" gap="xs" mt="md">
				<Button variant="default" onClick={load_current_config} disabled={applying}>
					{t("config.revert")}
				</Button>
				<Button onClick={apply} loading={applying}>
					{t("config.apply")}
				</Button>
			</Group>
		</Modal>
	);
}
