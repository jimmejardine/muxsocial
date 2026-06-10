import { Badge, Button, CloseButton, Group, Text, TextInput } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Virtuoso } from "react-virtuoso";
import { usePosts } from "../hooks/usePosts.ts";
import { networkColor } from "../theme/networkColors.ts";
import type { Post } from "../tools/Post.ts";
import type { SourceConfig, TimelineConfig } from "../tools/TimelineConfig.ts";
import { ConfirmModal } from "./ConfirmModal.tsx";
import { PostCard } from "./PostCard.tsx";
import classes from "./Timeline.module.css";

interface TimelineProps {
	timeline: TimelineConfig;
	index: number;
	on_remove: (id: string) => void;
	on_add_source: (id: string, address: string) => void;
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
export function Timeline({ timeline, index, on_remove, on_add_source, on_set_name }: TimelineProps) {
	const { t } = useTranslation();
	const [address, set_address] = useState("");
	// Local draft of the custom name; the effective name (default) is the placeholder.
	const [name_draft, set_name_draft] = useState(timeline.name ?? "");
	const [confirm_opened, confirm_handlers] = useDisclosure(false);
	const { posts, firstItemIndex, loading, reachedOldest, getMore } = usePosts(timeline.id);

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
				<Group gap="xs" wrap="nowrap">
					<Button size="xs" variant="light" onClick={getMore} loading={loading}>
						{t("timeline.get_more")}
					</Button>
					<CloseButton aria-label={t("timeline.remove")} title={t("timeline.remove")} onClick={confirm_handlers.open} />
				</Group>
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

			{timeline.sources.length > 0 && (
				<Group className={classes.sourceChips} gap={4} wrap="wrap">
					{timeline.sources.map((source) => (
						<SourceChip key={`${source.network}:${source.id}`} source={source} />
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
		</section>
	);
}

function SourceChip({ source }: { source: SourceConfig }) {
	return (
		<Badge size="sm" variant="light" radius="sm" color={networkColor(source.network)} title={`${source.network}: ${source.id}`}>
			{source.network}: {source.id}
		</Badge>
	);
}
