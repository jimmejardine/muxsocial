import { Stack } from "@mantine/core";
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

	const color = networkColor(post.source);
	// Two stacked copies of the translucent `-light` tint raise its opacity a little
	// (still see-through, so the sticky bar's backdrop blur keeps working).
	const source_bar_style = {
		backgroundColor: `var(--mantine-color-${color}-light)`,
		backgroundImage: `linear-gradient(var(--mantine-color-${color}-light), var(--mantine-color-${color}-light))`,
		color: `var(--mantine-color-${color}-light-color)`,
	};
	const source_label = <span className={classes.sourceLabel}>{author}</span>;
	const source_date = (
		<span className={classes.sourceDate}>
			<RelativeTimeAgo date_millis={post.created_at_millis} />
		</span>
	);

	return (
		<div className={classes.card}>
			{post.post_url ? (
				<a className={classes.sourceBar} style={source_bar_style} href={post.post_url} target="_blank" rel="noreferrer" title={t("post.open_original")}>
					{source_label}
					{source_date}
				</a>
			) : (
				<div className={classes.sourceBar} style={source_bar_style} title={post.source}>
					{source_label}
					{source_date}
				</div>
			)}
			<Stack gap={4} className={classes.content}>
				<div className={classes.body}>
					<PostBody content_html={post.content_html} />
				</div>
				<PostMedia media={post.media} />
			</Stack>
		</div>
	);
}
