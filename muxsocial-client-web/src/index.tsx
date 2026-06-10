import "./index.css";
import "@mantine/core/styles.css";
import "@mantine/notifications/styles.css";
import { Center, Loader, MantineProvider, Title } from "@mantine/core";
import { Notifications } from "@mantine/notifications";
import React, { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { HashRouter, Route, Routes } from "react-router";
import { Muxsocial } from "./Muxsocial.ts";

function HomePage() {
	const [greeting_message, set_greeting_message] = useState<string | null>(null);

	useEffect(() => {
		let cancelled = false;
		(async () => {
			const muxsocial_client_proxy = await Muxsocial.create();
			const composed_greeting_message = await muxsocial_client_proxy.compose_greeting("mux.social");
			if (!cancelled) {
				set_greeting_message(composed_greeting_message);
			}
		})();
		return () => {
			cancelled = true;
		};
	}, []);

	return <Center h="100dvh">{greeting_message ? <Title order={2}>{greeting_message}</Title> : <Loader />}</Center>;
}

const root_element = document.getElementById("root");
if (!root_element) {
	throw new Error("Root element #root not found");
}

createRoot(root_element).render(
	<React.StrictMode>
		<MantineProvider defaultColorScheme="auto">
			<Notifications />
			<HashRouter>
				<Routes>
					<Route path="/" element={<HomePage />} />
				</Routes>
			</HashRouter>
		</MantineProvider>
	</React.StrictMode>,
);
