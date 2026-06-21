import { Alert, Button, Divider, Group, Modal, PasswordInput, Stack, Text, TextInput } from "@mantine/core";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useMuxsocial } from "../tools/MuxsocialContext.tsx";
import { oauth_redirect_uri, open_oauth_popup } from "../tools/OAuthPopup.ts";
import type { AccountView, SourceNetwork } from "../tools/PostingTypes.ts";
import { Toast } from "../tools/Toast.ts";

interface AccountsModalProps {
	opened: boolean;
	onClose: () => void;
	/** Called after accounts change (add/remove) so other open views (the compose
	 * dialog) can refresh their account list/unlock state. */
	onAccountsChanged?: () => void;
}

function message_of(error: unknown): string {
	return error instanceof Error ? error.message : String(error);
}

/** How a network is connected. `secret` pastes a key; `oauth` runs the popup flow. */
type AddMode = "secret" | "oauth";

/** The four networks, in tagline order. */
const NETWORKS: { network: SourceNetwork; mode: AddMode; enabled: boolean }[] = [
	{ network: "Hashiverse", mode: "secret", enabled: true },
	{ network: "Nostr", mode: "secret", enabled: true },
	{ network: "Mastodon", mode: "oauth", enabled: true },
	{ network: "Bluesky", mode: "oauth", enabled: true },
];

/**
 * The Bluesky OAuth `client_id`. On an http loopback dev origin (127.0.0.1) it is
 * empty → the ATProto localhost dev client. Otherwise it is the URL of the static
 * `client-metadata-bluesky.json` served at this origin (which the auth server
 * fetches, so the file's `client_id`/`redirect_uris` must match this origin).
 */
function bluesky_client_id(): string {
	const origin = window.location.origin;
	if (origin.startsWith("http://127.0.0.1") || origin.startsWith("http://localhost") || origin.startsWith("http://[::1]")) {
		return "";
	}
	return `${origin}/client-metadata-bluesky.json`;
}

/**
 * The "My accounts" dialog: an Add button per network (icons supplied later) and
 * a removable list of connected accounts. Hashiverse (keyphrase) and nostr (nsec)
 * are added by pasting a secret; Mastodon connects via OAuth in a popup. Bluesky
 * (OAuth) is disabled until that flow ships.
 */
export function AccountsModal({ opened, onClose, onAccountsChanged }: AccountsModalProps) {
	const { t } = useTranslation();
	const muxsocial = useMuxsocial();

	const [accounts, set_accounts] = useState<AccountView[] | null>(null);
	const [unlocked, set_unlocked] = useState(false);
	const [adding, set_adding] = useState<SourceNetwork | null>(null);
	const [error, set_error] = useState<string | null>(null);

	// Add-form fields.
	const [secret, set_secret] = useState("");
	const [label, set_label] = useState("");
	const [instance, set_instance] = useState("");
	const [master_password, set_master_password] = useState("");
	const [busy, set_busy] = useState(false);

	// Reload accounts + unlock state after a mutation (add/remove).
	const reload = async () => {
		if (!muxsocial) return;
		const [list, is_unlocked] = await Promise.all([muxsocial.list_accounts() as Promise<AccountView[]>, muxsocial.is_unlocked() as Promise<boolean>]);
		set_accounts(list);
		set_unlocked(is_unlocked);
	};

	useEffect(() => {
		if (!opened || !muxsocial) return;
		let cancelled = false;
		(async () => {
			try {
				const [list, is_unlocked] = await Promise.all([muxsocial.list_accounts() as Promise<AccountView[]>, muxsocial.is_unlocked() as Promise<boolean>]);
				if (!cancelled) {
					set_accounts(list);
					set_unlocked(is_unlocked);
					set_error(null);
				}
			} catch (caught) {
				if (!cancelled) set_error(message_of(caught));
			}
		})();
		return () => {
			cancelled = true;
		};
	}, [opened, muxsocial]);

	const add_mode = (network: SourceNetwork): AddMode => NETWORKS.find((entry) => entry.network === network)?.mode ?? "secret";

	const start_add = (network: SourceNetwork) => {
		set_adding(network);
		set_secret("");
		set_label("");
		set_instance("");
		set_master_password("");
		set_error(null);
	};

	const cancel_add = () => {
		set_adding(null);
		set_secret("");
		set_label("");
		set_instance("");
		set_master_password("");
	};

	const submit_add = async () => {
		if (!muxsocial || !adding) return;
		set_busy(true);
		set_error(null);
		try {
			// When the session is already unlocked the password arg is ignored.
			const password = unlocked ? "" : master_password;
			if (adding === "Nostr") {
				await muxsocial.add_nostr_account(secret.trim(), password);
			} else if (adding === "Hashiverse") {
				await muxsocial.add_hashiverse_account(secret.trim(), label.trim(), password);
			} else if (adding === "Mastodon" || adding === "Bluesky") {
				const client_id = adding === "Bluesky" ? bluesky_client_id() : "";
				const begin = (await muxsocial.begin_oauth(adding, instance.trim(), oauth_redirect_uri(), client_id)) as { authorize_url: string; oauth_flow_id: string };
				const callback_query = await open_oauth_popup(begin.authorize_url);
				await muxsocial.complete_oauth(begin.oauth_flow_id, callback_query, password);
			}
			await reload();
			cancel_add();
			onAccountsChanged?.();
			Toast.success(t("toast.account_added"));
		} catch (caught) {
			set_error(message_of(caught));
		} finally {
			set_busy(false);
		}
	};

	const remove = async (account_id: string) => {
		if (!muxsocial) return;
		try {
			set_accounts((await muxsocial.remove_account(account_id)) as AccountView[]);
			onAccountsChanged?.();
			Toast.success(t("toast.account_removed"));
		} catch (caught) {
			Toast.error(t("toast.error_account_remove", { message: message_of(caught) }));
		}
	};

	const adding_mode = adding ? add_mode(adding) : null;
	// Whether the master-password field is required to submit (only when locked).
	const needs_password = !unlocked;
	const submit_disabled =
		busy || (needs_password && master_password.length === 0) || (adding_mode === "secret" && secret.trim().length === 0) || (adding === "Mastodon" && instance.trim().length === 0);

	return (
		<Modal opened={opened} onClose={onClose} title={t("accounts.title")} size="lg" centered>
			<Stack gap="sm">
				<Group gap="xs">
					{NETWORKS.map(({ network, enabled }) => (
						<Button key={network} size="xs" variant="default" disabled={!enabled} title={enabled ? undefined : t("accounts.coming_soon")} onClick={() => start_add(network)}>
							{t("accounts.add", { network })}
						</Button>
					))}
				</Group>

				{adding && (
					<Stack gap="xs" p="sm" style={{ border: "1px solid var(--mantine-color-default-border)", borderRadius: 6 }}>
						<Text size="sm" fw={600}>
							{t("accounts.add", { network: adding })}
						</Text>

						{adding === "Nostr" && <PasswordInput data-autofocus label={t("accounts.nsec")} value={secret} onChange={(event) => set_secret(event.currentTarget.value)} />}

						{adding === "Hashiverse" && (
							<>
								<PasswordInput data-autofocus label={t("accounts.keyphrase")} value={secret} onChange={(event) => set_secret(event.currentTarget.value)} />
								<TextInput label={t("accounts.hashiverse_label")} value={label} onChange={(event) => set_label(event.currentTarget.value)} />
							</>
						)}

						{adding === "Mastodon" && (
							<TextInput data-autofocus label={t("accounts.instance_domain")} placeholder="mastodon.social" value={instance} onChange={(event) => set_instance(event.currentTarget.value)} />
						)}

						{adding === "Bluesky" && (
							<TextInput data-autofocus label={t("accounts.handle")} placeholder="you.bsky.social" value={instance} onChange={(event) => set_instance(event.currentTarget.value)} />
						)}

						{needs_password && (
							<PasswordInput
								label={t("accounts.master_password")}
								description={t("accounts.master_password_help")}
								value={master_password}
								onChange={(event) => set_master_password(event.currentTarget.value)}
							/>
						)}

						<Group justify="flex-end" gap="xs">
							<Button variant="default" size="xs" onClick={cancel_add}>
								{t("common.cancel")}
							</Button>
							<Button size="xs" onClick={submit_add} loading={busy} disabled={submit_disabled}>
								{adding_mode === "oauth" ? t("accounts.connect") : t("accounts.save")}
							</Button>
						</Group>
					</Stack>
				)}

				{error && <Alert color="red">{error}</Alert>}

				<Divider label={t("accounts.connected")} labelPosition="left" />

				{accounts === null || accounts.length === 0 ? (
					<Text size="sm" c="dimmed">
						{t("accounts.empty")}
					</Text>
				) : (
					<Stack gap="xs">
						{accounts.map((account) => (
							<Group key={account.account_id} justify="space-between" wrap="nowrap">
								<Text size="sm" truncate>
									<Text span fw={600}>
										{account.network}
									</Text>{" "}
									· {account.display_label}
								</Text>
								<Button size="compact-xs" variant="subtle" color="red" onClick={() => remove(account.account_id)}>
									{t("accounts.remove")}
								</Button>
							</Group>
						))}
					</Stack>
				)}
			</Stack>
		</Modal>
	);
}
