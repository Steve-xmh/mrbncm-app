import { useAtomValue } from "jotai";
import { useParams } from "react-router-dom";
import { ncmAPIAtom } from "../../ncm-api";
import { useEffect, useState } from "react";
import "./index.sass";
import { BarLoader } from "react-spinners";

export const PlaylistPage: React.FC = () => {
	const ncm = useAtomValue(ncmAPIAtom);
	const [playlist, setPlaylist] = useState({});
	const [playlistSongs, setPlaylistSongs] = useState<any[] | null>(null);
	const param = useParams();

	useEffect(() => {
		if (param.id && ncm) {
			// setPlaylistId(param.id);
			let canceled = false;
			(async () => {
				setPlaylist({});
				setPlaylistSongs(null);
				const res = await ncm.request(
					"https://music.163.com/eapi/v6/playlist/detail",
					JSON.stringify({
						id: param.id,
						n: 100000,
						s: 0,
					}),
				);
				if (!canceled) setPlaylist(res);
				const songsAmount = res?.playlist?.trackIds?.length ?? 0;
				const songsThreads = [];
				for (let i = 0; i < songsAmount; i += 1000) {
					const postData = [];
					for (let j = 0; j < Math.min(1000, songsAmount - i); j++) {
						postData.push({
							id: res?.playlist?.trackIds[i + j]?.id,
							v: 0,
						});
					}
					songsThreads.push(
						ncm.request(
							"https://music.163.com/eapi/v3/song/detail",
							JSON.stringify({
								c: JSON.stringify(postData),
								e_r: true,
							}),
						),
					);
				}
				const songs = (await Promise.all(songsThreads)).flatMap((v) => v.songs);
				console.log(songs);
				setPlaylistSongs(songs);
			})();
			return () => {
				canceled = true;
			};
		}
	}, [param.id, ncm]);

	return (
		<div className="playlist-page">
			<div className="playlist-top">
				<img
					width={256}
					height={256}
					alt="播放列表图片"
					className="playlist-cover-img"
					src={playlist?.playlist?.coverImgUrl || ""}
				/>
				<div className="playlist-info">
					<div className="playlist-name">{playlist?.playlist?.name || ""}</div>
					<div className="playlist-creator">
						<img
							width={32}
							height={32}
							alt="播放列表创建者头像"
							className="playlist-creator-avatar-img"
							src={playlist?.playlist?.creator?.avatarUrl || ""}
						/>
						<div>{playlist?.playlist?.creator?.nickname || ""}</div>
					</div>
					<div className="playlist-actions">
						<button className="playlist-play-btn" type="button">
							播放歌单
						</button>
					</div>
				</div>
			</div>
			<div className="playlist-songs">
				{playlistSongs ? (
					playlistSongs.map((v) => (
						<div className="song-btn" key={`playlist-songs-${v.id}`}>
							<img
								width={32}
								height={32}
								alt={`歌曲 ${v.name} 的专辑图片`}
								className="song-album-img"
								src={v?.al?.picUrl || ""}
								loading="lazy"
							/>
							<div>{v.name}</div>
						</div>
					))
				) : (
					<BarLoader />
				)}
			</div>
		</div>
	);
};
