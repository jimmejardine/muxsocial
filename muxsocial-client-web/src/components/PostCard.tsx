import { Stack, Text } from "@mantine/core";
import { useTranslation } from "react-i18next";
import { networkColor } from "../theme/networkColors.ts";
import type { Post } from "../tools/Post.ts";
import { RelativeTimeAgo } from "../tools/RelativeTimeAgo.tsx";
import { truncate_source_id } from "../tools/sourceLabel.ts";
import { PostBody } from "./PostBody.tsx";
import classes from "./PostCard.module.css";
import { PostMedia } from "./PostMedia.tsx";

interface PostCardProps {
	post: Post;
}

/**
 * One post in a timeline: a bar across the top in the source network's color
 * showing the poster (display name, or identifier when there is none) that links
 * to the original post when a permalink is available, then the sanitized body,
 * media, and time. The network is conveyed by the bar's color.
 */
export function PostCard({ post }: PostCardProps) {
	const { t } = useTranslation();
	const author = post.author_display_name ?? truncate_source_id(post.source, post.author_identifier);
	const timestamp = new Date(post.created_at_millis).toLocaleString();

	const color = networkColor(post.source);
	const source_bar_style = { backgroundColor: `var(--mantine-color-${color}-light)`, color: `var(--mantine-color-${color}-light-color)` };
	const source_label = <span className={classes.sourceLabel}>{author}</span>;

	return (
		<div className={classes.card}>
			{post.post_url ? (
				<a className={classes.sourceBar} style={source_bar_style} href={post.post_url} target="_blank" rel="noreferrer" title={t("post.open_original")}>
					{source_label}
				</a>
			) : (
				<div className={classes.sourceBar} style={source_bar_style} title={post.source}>
					{source_label}
				</div>
			)}
			<Stack gap={4} className={classes.content}>
				<div className={classes.body}>
					<PostBody content_html={post.content_html} />
				</div>
				<PostMedia media={post.media} />
				<Text size="xs" c="dimmed">
					{timestamp} · <RelativeTimeAgo date_millis={post.created_at_millis} />
				</Text>
			</Stack>
		</div>
	);
}
