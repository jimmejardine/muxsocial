/**
 * Composing and parsing the config-transfer JSON shown in the Config dialog.
 *
 * The config is one JSON document with two root elements:
 *
 * - `"timelines"` — the persisted timeline list, owned and validated by the Rust
 *   layer. It is treated as an opaque array here; the deep validation (and the
 *   all-or-nothing replace) happens in `TimelineRegistry::import_timelines_json`.
 * - `"settings"` — the GUI-selectable preferences (theme, language), owned by the
 *   GUI and persisted in localStorage by their respective providers.
 *
 * These functions are pure (the valid theme/language ids are passed in) so they
 * can be unit-tested without React or WASM.
 *
 * @module
 */

/** The GUI-selectable preferences carried in the config's "settings" root. */
export interface ConfigSettings {
	theme?: string;
	language?: string;
}

/** A parsed-and-validated config, split back into its two roots. */
export interface ParsedConfig {
	/** The "timelines" root re-serialized, ready for the WASM `import_timelines_json`. */
	timelines_json: string;
	settings: ConfigSettings;
}

/**
 * Compose the config text shown in the dialog: the Rust-exported timelines JSON
 * embedded under "timelines", plus the current GUI settings, pretty-printed.
 */
export function compose_config_text(timelines_json: string, settings: ConfigSettings): string {
	return JSON.stringify({ timelines: JSON.parse(timelines_json), settings }, null, 2);
}

/**
 * Parse and validate pasted config text. Throws an `Error` with a human-readable
 * message when the text is not valid JSON, the roots are missing or of the wrong
 * shape, or a setting names an unknown theme/language. The timelines array is
 * only checked to *be* an array — its contents are validated by the Rust import.
 */
export function parse_config_text(text: string, valid_theme_ids: string[], valid_language_codes: string[]): ParsedConfig {
	let root: unknown;
	try {
		root = JSON.parse(text);
	} catch (parse_error) {
		throw new Error(`not valid JSON: ${parse_error instanceof Error ? parse_error.message : String(parse_error)}`);
	}
	if (root === null || typeof root !== "object" || Array.isArray(root)) {
		throw new Error('the config root must be a JSON object with "timelines" and "settings"');
	}
	const config = root as Record<string, unknown>;

	if (!Array.isArray(config.timelines)) {
		throw new Error('"timelines" must be a JSON array');
	}

	if (config.settings === null || typeof config.settings !== "object" || Array.isArray(config.settings)) {
		throw new Error('"settings" must be a JSON object');
	}
	const settings_object = config.settings as Record<string, unknown>;

	const settings: ConfigSettings = {};
	if (settings_object.theme !== undefined) {
		if (typeof settings_object.theme !== "string" || !valid_theme_ids.includes(settings_object.theme)) {
			throw new Error(`unknown theme ${JSON.stringify(settings_object.theme)}; valid themes: ${valid_theme_ids.join(", ")}`);
		}
		settings.theme = settings_object.theme;
	}
	if (settings_object.language !== undefined) {
		if (typeof settings_object.language !== "string" || !valid_language_codes.includes(settings_object.language)) {
			throw new Error(`unknown language ${JSON.stringify(settings_object.language)}; valid languages: ${valid_language_codes.join(", ")}`);
		}
		settings.language = settings_object.language;
	}

	return { timelines_json: JSON.stringify(config.timelines), settings };
}
