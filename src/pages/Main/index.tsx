import { useAtomValue } from "jotai";
import { ncmCookieAtom } from "../../ncm-cookie";
import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import "./index.sass";
import { ncmAPIAtom } from "../../ncm-api";

export const MainPage: React.FC = () => {
	const ncmCookies = useAtomValue(ncmCookieAtom);
	const navigate = useNavigate();
	const ncmAPI = useAtomValue(ncmAPIAtom);

	useEffect(() => {
		if (ncmCookies.length === 0) {
			navigate("/login");
		} else {
			let canceled = false;
			(async () => {
				// const res = await ncmAPI.request("https://music.163.com/eapi/v2/banner/get", JSON.stringify({
				//     clientType: "pc"
				// }));
				// console.log(res);
				const res = await ncmAPI.request(
					"https://music.163.com/api/nuser/account/get",
					"{}",
				);
				console.log(res);
			})();
			return () => {
				canceled = true;
			};
		}
	}, []);

	return <div>欢迎来到 MRBNCM App！</div>;
};
