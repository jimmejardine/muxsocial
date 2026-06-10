import { Stack, Text } from "@mantine/core";
import { networkColor } from "../theme/networkColors.ts";
import type { Post } from "../tools/Post.ts";
import { RelativeTimeAgo } from "../tools/RelativeTimeAgo.tsx";
import { PostBody } from "./PostBody.tsx";
import classes from "./PostCard.module.css";

interface PostCardProps {
	post: Post;
}

/**
 * One post in a timeline: a thin left bar in the source network's color (with the
 * network name written vertically), then the author, sanitized body, and time.
 */
export function PostCard({ post }: PostCardProps) {
	const author = post.author_display_name ?? post.author_identifier;
	const timestamp = new Date(post.created_at_millis).toLocaleString();

	const color = networkColor(post.source);
	return (
		<div className={classes.card}>
			<div className={classes.sourceBar} style={{ backgroundColor: `var(--mantine-color-${color}-light)`, color: `var(--mantine-color-${color}-light-color)` }} title={post.source}>
				<span className={classes.sourceLabel}>{post.source}</span>
			</div>
			<Stack gap={4} className={classes.content}>
				<Text size="sm" fw={600} truncate>
					{author}
				</Text>
				<div className={classes.body}>
					<PostBody content_html={post.content_html} />
				</div>
				<Text size="xs" c="dimmed">
					{timestamp} · <RelativeTimeAgo date_millis={post.created_at_millis} />
				</Text>
			</Stack>
		</div>
	);
}
