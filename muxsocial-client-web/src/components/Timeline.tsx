import { CloseButton, Group, Text, TextInput } from "@mantine/core";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Virtuoso } from "react-virtuoso";
import type { SourceConfig, TimelineConfig } from "../tools/TimelineConfig.ts";
import classes from "./Timeline.module.css";

interface TimelineProps {
	timeline: TimelineConfig;
	index: number;
	on_remove: (id: string) => void;
	on_add_source: (id: string, address: string) => void;
	on_set_name: (id: string, name: string) => void;
}

/**
 * A single timeline column: an editable name + remove button, an address textbox
 * (paste + Enter adds a source to this timeline), and a `react-virtuoso` body that
 * lists the timeline's sources. Every action is addressed by the timeline's GUID
 * (`timeline.id`). All timeline state — the name and sources — comes from the Rust
 * snapshot; this component never derives it (the name's source-derived default is
 * `timeline.display_name`, computed in Rust).
 */
export function Timeline({ timeline, index, on_remove, on_add_source, on_set_name }: TimelineProps) {
	const { t } = useTranslation();
	const [address, set_address] = useState("");
	// Local draft of the custom name; the effective name (default) is the placeholder.
	const [name_draft, set_name_draft] = useState(timeline.name ?? "");

	// display_name is empty only for a nameless, sourceless timeline -> index fallback.
	const default_title = timeline.display_name || t("timeline.title", { number: index + 1 });

	const submit_address = () => {
		const trimmed_address = address.trim();
		if (trimmed_address.length > 0) {
			on_add_source(timeline.id, trimmed_address);
			set_address("");
		}
	};

	const commit_name = () => {
		if (name_draft !== (timeline.name ?? "")) {
			on_set_name(timeline.id, name_draft);
		}
	};

	return (
		<section className={classes.timeline} aria-label={default_title}>
			<Group className={classes.miniToolbar} justify="space-between" wrap="nowrap" gap="xs">
				<TextInput
					className={classes.nameInput}
					variant="unstyled"
					size="sm"
					aria-label={t("timeline.name_label")}
					value={name_draft}
					placeholder={default_title}
					styles={{ input: { fontWeight: 600 } }}
					onChange={(event) => set_name_draft(event.currentTarget.value)}
					onBlur={commit_name}
					onKeyDown={(event) => {
						if (event.key === "Enter") {
							event.preventDefault();
							event.currentTarget.blur();
						}
					}}
				/>
				<CloseButton aria-label={t("timeline.remove")} title={t("timeline.remove")} onClick={() => on_remove(timeline.id)} />
			</Group>

			<div className={classes.addressBar}>
				<TextInput
					size="xs"
					placeholder={t("timeline.address_placeholder")}
					value={address}
					onChange={(event) => set_address(event.currentTarget.value)}
					onKeyDown={(event) => {
						if (event.key === "Enter") {
							event.preventDefault();
							submit_address();
						}
					}}
				/>
			</div>

			<div className={classes.body}>
				<Virtuoso
					data={timeline.sources}
					style={{ height: "100%" }}
					itemContent={(_index: number, source: SourceConfig) => <SourceRow source={source} />}
					components={{
						EmptyPlaceholder: () => (
							<Text c="dimmed" size="sm" p="sm">
								{t("timeline.no_sources")}
							</Text>
						),
					}}
				/>
			</div>
		</section>
	);
}

function SourceRow({ source }: { source: SourceConfig }) {
	return (
		<Group gap="xs" px="sm" py={6} wrap="nowrap" className={classes.sourceRow}>
			<Text size="xs" c="dimmed" fw={600}>
				{source.network}
			</Text>
			<Text size="sm" truncate>
				{source.id}
			</Text>
		</Group>
	);
}
