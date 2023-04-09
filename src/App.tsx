import { useSetAtom } from "jotai";
import logo from "./assets/logo.svg";
import { setNCMCookieAtom } from "./ncm-cookie";
import { useState } from "react";

function App() {
	const setNCMCookies = useSetAtom(setNCMCookieAtom);
	const [cookie, setCookie] = useState("");
	return (
		<div className="container">
			<div className="login-page">
				<div>
					<h1>Welcome to MRBNCM App!</h1>
					<div>Please enter your auth string to continue:</div>
				</div>
				<div>
					<textarea
						onChange={(e) => setCookie(e.target.value)}
						value={cookie}
					/>
					<button
						onClick={() => {
							setNCMCookies(JSON.parse(cookie));
						}}
					>
						Login
					</button>
				</div>
			</div>
		</div>
	);
}

export default App;
