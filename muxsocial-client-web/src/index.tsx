import "./index.css";
import "@mantine/core/styles.css";
import "@mantine/notifications/styles.css";
import "./i18n/i18n.ts";
import { Notifications } from "@mantine/notifications";
import React from "react";
import { createRoot } from "react-dom/client";
import { HashRouter, Route, Routes } from "react-router";
import { App } from "./App.tsx";
import { MuxThemeProvider } from "./theme/MuxThemeProvider.tsx";

const root_element = document.getElementById("root");
if (!root_element) {
	throw new Error("Root element #root not found");
}

createRoot(root_element).render(
	<React.StrictMode>
		<MuxThemeProvider>
			<Notifications position="bottom-center" />
			<HashRouter>
				<Routes>
					<Route path="/" element={<App />} />
				</Routes>
			</HashRouter>
		</MuxThemeProvider>
	</React.StrictMode>,
);
