import { Badge, Button, CloseButton, Group, Text, TextInput, UnstyledButton } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Virtuoso } from "react-virtuoso";
import { usePosts } from "../hooks/usePosts.ts";
import { networkColor } from "../theme/networkColors.ts";
import type { Post } from "../tools/Post.ts";
import { truncate_source_id } from "../tools/sourceLabel.ts";
import type { SourceConfig, TimelineConfig } from "../tools/TimelineConfig.ts";
import { Toast } from "../tools/Toast.ts";
import { ConfirmModal } from "./ConfirmModal.tsx";
import { PostCard } from "./PostCard.tsx";
import classes from "./Timeline.module.css";

interface TimelineProps {
	timeline: TimelineConfig;
	index: number;
	/** Pulse the address box for attention (the lone timeline has no sources yet). */
	highlight_add_source?: boolean;
	on_remove: (id: string) => void;
	on_add_source: (id: string, address: string) => void;
	on_remove_source: (id: string, network: string, source_id: string) => void;
	on_set_name: (id: string, name: string) => void;
}

/**
 * A single timeline column: an editable name + a "get more posts" button + remove
 * button, an address textbox (paste + Enter adds a source), a compact row of the
 * timeline's sources, and a `react-virtuoso` body that lists the merged posts.
 * Every action is addressed by the timeline's GUID (`timeline.id`). The name and
 * sources come from the Rust snapshot; the posts come from {@link usePosts}, which
 * holds the rendered window and pages it via the Rust client.
 */
export function Timeline({ timeline, index, highlight_add_source, on_remove, on_add_source, on_remove_source, on_set_name }: TimelineProps) {
	const { t } = useTranslation();
	const [address, set_address] = useState("");
	// Local draft of the custom name; the effective name (default) is the placeholder.
	const [name_draft, set_name_draft] = useState(timeline.name ?? "");
	const [confirm_opened, confirm_handlers] = useDisclosure(false);
	// The source pending removal (drives the source-remove confirm dialog), or null.
	const [source_to_remove, set_source_to_remove] = useState<SourceConfig | null>(null);
	// A stable key of the current sources; changes when an address is added/removed,
	// driving usePosts to auto-fetch (the truncated source_summary below is display only).
	const sources_signature = timeline.sources.map((source) => `${source.network}:${source.id}`).join(",");
	const { posts, firstItemIndex, loading, reachedOldest, getMore } = usePosts(timeline.id, sources_signature);

	// The default title (placeholder for an unnamed timeline) summarizes the sources
	// with the same id shortener as the chips; a custom name wins, index is the fallback.
	const source_summary = timeline.sources.map((source) => truncate_source_id(source.network, source.id)).join(", ");
	const default_title = timeline.name || source_summary || t("timeline.title", { number: index + 1 });

	const submit_address = () => {
		const trimmed_address = address.trim();
		if (trimmed_address.length > 0) {
			on_add_source(timeline.id, trimmed_address);
			set_address("");
		}
	};

	// An empty timeline has nothing to lose, so skip the confirm dialog; otherwise ask.
	const request_remove = () => {
		if (timeline.sources.length === 0) {
			on_remove(timeline.id);
		} else {
			confirm_handlers.open();
		}
	};

	const commit_name = () => {
		if (name_draft !== (timeline.name ?? "")) {
			on_set_name(timeline.id, name_draft);
		}
	};

	// Copy a source's full address (not the truncated chip text) to the clipboard.
	const copy_address = (source_id: string) => {
		void (async () => {
			try {
				await navigator.clipboard.writeText(source_id);
				Toast.success(t("toast.address_copied"));
			} catch (err) {
				Toast.error(t("toast.error_copy", { message: err instanceof Error ? err.message : String(err) }));
			}
		})();
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
				<Group gap="xs" wrap="nowrap">
					<Button size="xs" variant="light" onClick={getMore} loading={loading}>
						{t("timeline.get_more")}
					</Button>
					<CloseButton aria-label={t("timeline.remove")} title={t("timeline.remove")} onClick={request_remove} />
				</Group>
			</Group>

			<div className={classes.addressBar}>
				<TextInput
					size="xs"
					classNames={{ input: highlight_add_source ? "mux-throb" : undefined }}
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

			{timeline.sources.length > 0 && (
				<Group className={classes.sourceChips} gap={4} wrap="wrap">
					{timeline.sources.map((source) => (
						<SourceChip
							key={`${source.network}:${source.id}`}
							source={source}
							on_copy={() => copy_address(source.id)}
							on_remove={() => set_source_to_remove(source)}
							remove_label={t("timeline.remove_source")}
						/>
					))}
				</Group>
			)}

			<div className={classes.body}>
				<Virtuoso
					data={posts}
					firstItemIndex={firstItemIndex}
					style={{ height: "100%" }}
					computeItemKey={(_index: number, post: Post) => post.source_post_id}
					itemContent={(_index: number, post: Post) => <PostCard post={post} />}
					components={{
						EmptyPlaceholder: () => (
							<Text c="dimmed" size="sm" p="sm">
								{timeline.sources.length === 0 ? t("timeline.no_sources") : t("timeline.no_posts")}
							</Text>
						),
						Footer: () =>
							reachedOldest && posts.length > 0 ? (
								<Text c="dimmed" size="xs" ta="center" p="sm">
									{t("timeline.reached_oldest")}
								</Text>
							) : null,
					}}
				/>
			</div>

			<ConfirmModal
				opened={confirm_opened}
				title={t("timeline.remove_confirm_title")}
				message={t("timeline.remove_confirm_message")}
				confirmLabel={t("common.remove")}
				onConfirm={() => {
					on_remove(timeline.id);
					confirm_handlers.close();
				}}
				onClose={confirm_handlers.close}
			/>

			<ConfirmModal
				opened={source_to_remove !== null}
				title={t("timeline.remove_source_confirm_title")}
				message={t("timeline.remove_source_confirm_message", { source: source_to_remove ? `${source_to_remove.network}: ${source_to_remove.id}` : "" })}
				confirmLabel={t("common.remove")}
				onConfirm={() => {
					if (source_to_remove) {
						on_remove_source(timeline.id, source_to_remove.network, source_to_remove.id);
					}
					set_source_to_remove(null);
				}}
				onClose={() => set_source_to_remove(null)}
			/>
		</section>
	);
}

function SourceChip({ source, on_copy, on_remove, remove_label }: { source: SourceConfig; on_copy: () => void; on_remove: () => void; remove_label: string }) {
	return (
		<Badge
			size="sm"
			variant="light"
			radius="sm"
			color={networkColor(source.network)}
			rightSection={<CloseButton size="xs" aria-label={remove_label} title={remove_label} onClick={on_remove} />}
		>
			{/* Click the label to copy the full address; the full id is also the tooltip. */}
			<UnstyledButton onClick={on_copy} title={`${source.network}: ${source.id}`} style={{ font: "inherit", color: "inherit", cursor: "pointer" }}>
				{source.network}: {truncate_source_id(source.network, source.id)}
			</UnstyledButton>
		</Badge>
	);
}
