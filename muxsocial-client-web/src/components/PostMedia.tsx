import { Text } from "@mantine/core";
import type { PostMedia as PostMediaItem } from "../tools/Post.ts";
import classes from "./PostMedia.module.css";

/** Render a post's attached media (images, video, link cards) below its body. */
export function PostMedia({ media }: { media: PostMediaItem[] }) {
	if (media.length === 0) {
		return null;
	}
	return (
		<div className={classes.media}>
			{media.map((item) => (
				<MediaItem key={`${item.kind}:${item.url}`} item={item} />
			))}
		</div>
	);
}

function MediaItem({ item }: { item: PostMediaItem }) {
	switch (item.kind) {
		case "image":
			return (
				<a href={item.url} target="_blank" rel="noreferrer" className={classes.imageLink}>
					<img className={classes.image} src={item.url} alt={item.alt ?? ""} loading="lazy" />
				</a>
			);
		case "video":
			return (
				// biome-ignore lint/a11y/useMediaCaption: source-network videos rarely carry caption tracks
				<video className={classes.video} src={item.url} poster={item.poster ?? undefined} controls preload="none" />
			);
		case "link_card":
			return (
				<a href={item.url} target="_blank" rel="noreferrer" className={classes.linkCard}>
					{item.thumbnail_url ? <img className={classes.linkCardThumb} src={item.thumbnail_url} alt="" loading="lazy" /> : null}
					<div className={classes.linkCardBody}>
						{item.title ? (
							<Text size="sm" fw={600} lineClamp={2}>
								{item.title}
							</Text>
						) : null}
						{item.description ? (
							<Text size="xs" c="dimmed" lineClamp={2}>
								{item.description}
							</Text>
						) : null}
						<Text size="xs" c="dimmed" truncate>
							{link_hostname(item.url)}
						</Text>
					</div>
				</a>
			);
	}
}

/** The host of `url` for the link-card footer, falling back to the raw URL. */
function link_hostname(url: string): string {
	try {
		return new URL(url).hostname;
	} catch {
		return url;
	}
}
