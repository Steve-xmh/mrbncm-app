import { useAtomValue, useSetAtom } from "jotai";
import { ncmCookieAtom, setNCMCookieAtom } from "../../ncm-cookie";
import { useEffect, useState } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";

export const LoginPage: React.FC = () => {
	const setNCMCookies = useSetAtom(setNCMCookieAtom);
	const ncmCookies = useAtomValue(ncmCookieAtom);
	const [searchParams] = useSearchParams();
	const navigate = useNavigate();
	const [cookie, setCookie] = useState("");

	useEffect(() => {
		if (ncmCookies.length > 0 && !searchParams.get("reset")) {
			navigate("/");
		}
	}, [ncmCookies]);

	return (
		<div className="login-page">
			<div>
				<h1>Welcome to MRBNCM App!</h1>
				<div>Please enter your auth string to continue:</div>
			</div>
			<div>
				<textarea onChange={(e) => setCookie(e.target.value)} value={cookie} />
				<button
					onClick={() => {
						setNCMCookies(JSON.parse(cookie));
					}}
				>
					Login
				</button>
			</div>
		</div>
	);
};
