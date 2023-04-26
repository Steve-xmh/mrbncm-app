import { Provider } from "jotai";
import React, { Suspense } from "react";
import { RouterProvider } from "react-router-dom";
import ReactDOM from "react-dom/client";
import "./index.sass";
import "./scroll-bar.css";
import * as NCMAPI from "./ncm-api";
import * as TAPI from "./tauri-api";
import { router } from "./pages";

window.ncmapi = NCMAPI;
window.tapi = TAPI;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
	<React.StrictMode>
		<Provider>
			<Suspense>
				<RouterProvider router={router} />
			</Suspense>
		</Provider>
	</React.StrictMode>,
);
