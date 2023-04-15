import { Provider } from "jotai";
import React, { Suspense } from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.sass";
import "./scroll-bar.css";
import * as NCMAPI from "./ncm-api";
import * as TAPI from "./tauri-api";

window.ncmapi = NCMAPI;
window.tapi = TAPI;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
	<React.StrictMode>
		<Provider>
			<Suspense>
				<App />
			</Suspense>
		</Provider>
	</React.StrictMode>,
);
