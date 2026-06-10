import { Group, Stack, Text } from "@mantine/core";
import type { Post } from "../tools/Post.ts";
import { PostBody } from "./PostBody.tsx";
import classes from "./PostCard.module.css";

interface PostCardProps {
	post: Post;
}

/** One post in a timeline: author, source network, sanitized body, and time. */
export function PostCard({ post }: PostCardProps) {
	const author = post.author_display_name ?? post.author_identifier;
	const timestamp = new Date(post.created_at_millis).toLocaleString();

	return (
		<Stack gap={4} className={classes.card}>
			<Group gap="xs" justify="space-between" wrap="nowrap">
				<Text size="sm" fw={600} truncate>
					{author}
				</Text>
				<Text size="xs" c="dimmed" style={{ flexShrink: 0 }}>
					{post.source}
				</Text>
			</Group>
			<div className={classes.body}>
				<PostBody content_html={post.content_html} />
			</div>
			<Text size="xs" c="dimmed">
				{timestamp}
			</Text>
		</Stack>
	);
}
