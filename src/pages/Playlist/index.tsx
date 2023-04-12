import { useAtomValue } from "jotai";
import { useParams } from "react-router-dom";
import { getSongDetailAtom, ncmAPIAtom } from "../../ncm-api";
import { useEffect, useState } from "react";
import "./index.sass";
import { BarLoader } from "react-spinners";
import { sendMsgToAudioThread } from "../../tauri-api";

export const PlaylistPage: React.FC = () => {
	const ncm = useAtomValue(ncmAPIAtom);
	const getSongDetail = useAtomValue(getSongDetailAtom);
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
				const ids = res?.playlist?.trackIds?.map((v) => v.id) ?? [];
				const songs = await getSongDetail(ids);
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
						<button
							className="playlist-play-btn"
							type="button"
							onClick={async () => {
								if (playlistSongs) {
									await sendMsgToAudioThread("setPlaylist", {
										songs: playlistSongs.map((v, i) => ({
											ncmId: String(v.id),
											localFile: "",
											duration: 0,
											origOrder: i,
										})),
									});
									await sendMsgToAudioThread("nextSong");
								}
							}}
						>
							播放歌单
						</button>
					</div>
				</div>
			</div>
			<div className="playlist-songs">
				{playlistSongs ? (
					playlistSongs.map((v, i) => (
						<div
							className="song-btn"
							key={`playlist-songs-${v.id}`}
							onDoubleClick={async () => {
								await sendMsgToAudioThread("setPlaylist", {
									songs: playlistSongs.map((v, i) => ({
										ncmId: String(v.id),
										localFile: "",
										duration: 0,
										origOrder: i,
									})),
								});
								await sendMsgToAudioThread("jumpToSong", {
									songIndex: i,
								});
							}}
						>
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
