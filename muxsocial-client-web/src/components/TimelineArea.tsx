import { Text } from "@mantine/core";
import { Timeline, type TimelineDescriptor } from "./Timeline.tsx";
import classes from "./TimelineArea.module.css";

interface TimelineAreaProps {
	timelines: TimelineDescriptor[];
	on_remove: (id: string) => void;
}

/**
 * The center area: timelines laid out side by side. Each timeline is at least
 * 500px wide (or the screen width when it is narrower); they grow to fill the
 * space when few, and scroll horizontally when too many fit.
 */
export function TimelineArea({ timelines, on_remove }: TimelineAreaProps) {
	if (timelines.length === 0) {
		return (
			<div className={classes.empty}>
				<Text c="dimmed">No timelines. Use "Add timeline" to create one.</Text>
			</div>
		);
	}

	return (
		<div className={classes.area}>
			{timelines.map((timeline, index) => (
				<Timeline key={timeline.id} timeline={timeline} index={index} on_remove={on_remove} />
			))}
		</div>
	);
}
