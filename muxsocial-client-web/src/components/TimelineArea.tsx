import { Text } from "@mantine/core";
import { useTranslation } from "react-i18next";
import type { TimelineConfig } from "../tools/TimelineConfig.ts";
import { Timeline } from "./Timeline.tsx";
import classes from "./TimelineArea.module.css";

interface TimelineAreaProps {
	timelines: TimelineConfig[];
	on_remove: (id: string) => void;
	on_add_source: (id: string, address: string) => void;
	on_remove_source: (id: string, network: string, source_id: string) => void;
	on_set_name: (id: string, name: string) => void;
}

/**
 * The center area: timelines laid out side by side. Each timeline is at least
 * 500px wide (or the screen width when it is narrower); they grow to fill the
 * space when few, and scroll horizontally when too many fit.
 */
export function TimelineArea({ timelines, on_remove, on_add_source, on_remove_source, on_set_name }: TimelineAreaProps) {
	const { t } = useTranslation();

	if (timelines.length === 0) {
		return (
			<div className={classes.empty}>
				<Text c="dimmed">{t("timeline_area.empty")}</Text>
			</div>
		);
	}

	return (
		<div className={classes.area}>
			{timelines.map((timeline, index) => (
				<Timeline key={timeline.id} timeline={timeline} index={index} on_remove={on_remove} on_add_source={on_add_source} on_remove_source={on_remove_source} on_set_name={on_set_name} />
			))}
		</div>
	);
}
