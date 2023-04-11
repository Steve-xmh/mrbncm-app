import { useAtomValue } from "jotai";
import { useParams, useSearchParams } from "react-router-dom";
import { ncmAPIAtom } from "../../ncm-api";
import { useEffect } from "react";

export const PlaylistPage: React.FC = () => {
	const ncm = useAtomValue(ncmAPIAtom);
	const param = useParams();

	useEffect(() => {
		(async () => {
			// https://music.163.com/api/v6/playlist/detail
			const res = await ncm.request(
				"https://music.163.com/eapi/v6/playlist/detail",
				JSON.stringify({
					id: param.id,
					n: 100000,
					s: 0,
				}),
			);
		})();
	}, [param.id]);

	return (
		<div className="playlist-page">
			<div></div>
		</div>
	);
};
