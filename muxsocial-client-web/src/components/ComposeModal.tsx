import { Alert, Button, Group, Modal, PasswordInput, Stack, Text, Textarea } from "@mantine/core";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useMuxsocial } from "../tools/MuxsocialContext.tsx";
import type { AccountView, PostResult } from "../tools/PostingTypes.ts";

/** localStorage key for the in-progress draft, kept across Cancel and reloads. */
const DRAFT_KEY = "muxsocial.compose_draft";

interface ComposeModalProps {
	opened: boolean;
	onClose: () => void;
	/** Open the "My accounts" dialog (also reachable from the hamburger). */
	onOpenAccounts: () => void;
	/** Bumped whenever accounts change (add/remove), so this open dialog refreshes
	 * its list/unlock state without needing to be closed and reopened. */
	accountsVersion?: number;
}

function message_of(error: unknown): string {
	return error instanceof Error ? error.message : String(error);
}

/**
 * The compose dialog: type a message and broadcast it to every authenticated
 * account at once. Post / Cancel / "My accounts" in the footer. Cancel keeps the
 * draft (persisted to localStorage) for continuation. The first time it is opened
 * with connected accounts in a session, it prompts for the master password to
 * unlock the encrypted credentials. Results are shown per-account inline.
 */
export function ComposeModal({ opened, onClose, onOpenAccounts, accountsVersion }: ComposeModalProps) {
	const { t } = useTranslation();
	const muxsocial = useMuxsocial();

	const [text, set_text] = useState<string>(() => localStorage.getItem(DRAFT_KEY) ?? "");
	const [accounts, set_accounts] = useState<AccountView[] | null>(null);
	const [unlocked, set_unlocked] = useState(false);
	const [master_password, set_master_password] = useState("");
	const [unlocking, set_unlocking] = useState(false);
	const [posting, set_posting] = useState(false);
	const [results, set_results] = useState<PostResult[] | null>(null);
	const [error, set_error] = useState<string | null>(null);

	// Refresh accounts + unlock state when the dialog opens, and whenever accounts
	// change (accountsVersion bump) while it stays open behind the My-accounts dialog.
	// biome-ignore lint/correctness/useExhaustiveDependencies: accountsVersion is an intentional refetch trigger (the body doesn't read it).
	useEffect(() => {
		if (!opened || !muxsocial) return;
		let cancelled = false;
		(async () => {
			try {
				const [list, is_unlocked] = await Promise.all([muxsocial.list_accounts() as Promise<AccountView[]>, muxsocial.is_unlocked() as Promise<boolean>]);
				if (!cancelled) {
					set_accounts(list);
					set_unlocked(is_unlocked);
					set_results(null);
					set_error(null);
				}
			} catch (caught) {
				if (!cancelled) set_error(message_of(caught));
			}
		})();
		return () => {
			cancelled = true;
		};
	}, [opened, muxsocial, accountsVersion]);

	const on_change_text = (value: string) => {
		set_text(value);
		localStorage.setItem(DRAFT_KEY, value);
	};

	const close = () => {
		// Cancel keeps the draft (already persisted); reset only transient UI.
		set_master_password("");
		set_results(null);
		set_error(null);
		onClose();
	};

	const unlock = async () => {
		if (!muxsocial) return;
		set_unlocking(true);
		set_error(null);
		try {
			await muxsocial.unlock_secrets(master_password);
			set_unlocked(true);
			set_master_password("");
		} catch (caught) {
			set_error(message_of(caught));
		} finally {
			set_unlocking(false);
		}
	};

	const post = async () => {
		if (!muxsocial) return;
		set_posting(true);
		set_error(null);
		try {
			const post_results = (await muxsocial.cross_post(text)) as PostResult[];
			set_results(post_results);
			// Clear the draft only if every account succeeded.
			if (post_results.length > 0 && post_results.every((result) => result.outcome.status === "published")) {
				localStorage.removeItem(DRAFT_KEY);
				set_text("");
			}
		} catch (caught) {
			set_error(message_of(caught));
		} finally {
			set_posting(false);
		}
	};

	const has_accounts = accounts !== null && accounts.length > 0;
	const can_post = unlocked && has_accounts && text.trim().length > 0 && !posting;

	return (
		<Modal opened={opened} onClose={close} title={t("compose.title")} size="lg" centered>
			<Stack gap="sm">
				{accounts !== null && !has_accounts && <Alert color="blue">{t("compose.no_accounts")}</Alert>}

				<Textarea data-autofocus autosize minRows={4} maxRows={12} placeholder={t("compose.placeholder")} value={text} onChange={(event) => on_change_text(event.currentTarget.value)} />

				{has_accounts && !unlocked && (
					<Group align="flex-end" gap="xs">
						<PasswordInput
							style={{ flex: 1 }}
							label={t("accounts.master_password")}
							value={master_password}
							onChange={(event) => set_master_password(event.currentTarget.value)}
							onKeyDown={(event) => {
								if (event.key === "Enter") unlock();
							}}
						/>
						<Button onClick={unlock} loading={unlocking} disabled={master_password.length === 0}>
							{t("accounts.unlock")}
						</Button>
					</Group>
				)}

				{has_accounts && unlocked && (
					<Text size="xs" c="dimmed">
						{t("compose.broadcast_to", { count: accounts.length })}
					</Text>
				)}

				{error && <Alert color="red">{error}</Alert>}

				{results && (
					<Stack gap={4}>
						{results.map((result) => (
							<Text key={`${result.network}:${result.account_label}`} size="sm" c={result.outcome.status === "published" ? "green" : "red"}>
								{result.network} · {result.account_label}:{" "}
								{result.outcome.status === "published" ? t("compose.result_success") : t("compose.result_failure", { message: result.outcome.error_message })}
							</Text>
						))}
					</Stack>
				)}

				<Group justify="space-between" mt="sm">
					<Button variant="subtle" onClick={onOpenAccounts}>
						{t("accounts.open")}
					</Button>
					<Group gap="xs">
						<Button variant="default" onClick={close}>
							{t("common.cancel")}
						</Button>
						<Button onClick={post} loading={posting} disabled={!can_post}>
							{t("compose.post")}
						</Button>
					</Group>
				</Group>
			</Stack>
		</Modal>
	);
}
