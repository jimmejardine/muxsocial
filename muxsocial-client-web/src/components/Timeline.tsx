import { CloseButton, Group, Stack, Text } from "@mantine/core";
import classes from "./Timeline.module.css";

export interface TimelineDescriptor {
	id: string;
}

interface TimelineProps {
	timeline: TimelineDescriptor;
	index: number;
	on_remove: (id: string) => void;
}

/**
 * A single timeline column: a small toolbar (title + remove button) over a
 * vertically-scrolling body. The body is a placeholder for now — real feed
 * content is wired in a later step.
 */
export function Timeline({ timeline, index, on_remove }: TimelineProps) {
	return (
		<section className={classes.timeline} aria-label={`Timeline ${index + 1}`}>
			<Group className={classes.miniToolbar} justify="space-between" wrap="nowrap" gap="xs">
				<Text fw={600} size="sm" truncate>
					Timeline {index + 1}
				</Text>
				<CloseButton aria-label="Remove timeline" title="Remove timeline" onClick={() => on_remove(timeline.id)} />
			</Group>
			<div className={classes.body}>
				<Stack gap="xs">
					<Text c="dimmed" size="sm">
						No source configured yet.
					</Text>
				</Stack>
			</div>
		</section>
	);
}
