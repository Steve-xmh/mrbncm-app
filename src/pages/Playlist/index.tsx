import { useAtomValue } from "jotai";
import { useParams } from "react-router-dom";
import { NCMSongDetail, getSongDetailAtom, ncmAPIAtom } from "../../ncm-api";
import { useEffect, useMemo, useState } from "react";
import "./index.sass";
import { BarLoader } from "react-spinners";
import { sendMsgToAudioThread } from "../../tauri-api";
import { LazyImage } from "../../components/LazyImage";
import { formatDurationCN } from "../../utils/format";
import { PlaylistView } from "../../components/PlaylistView";

export const PlaylistPage: React.FC = () => {
	const ncm = useAtomValue(ncmAPIAtom);
	const getSongDetail = useAtomValue(getSongDetailAtom);
	const [playlist, setPlaylist] = useState({});
	const [playlistSongs, setPlaylistSongs] = useState<NCMSongDetail[] | null>(
		null,
	);
	const totalDuration = useMemo(
		() => playlistSongs?.reduce((pv, cv) => pv + cv.dt, 0),
		[playlistSongs],
	);
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
					{
						id: param.id,
						n: 100000,
						s: 0,
					},
				);
				if (!canceled) setPlaylist(res);
				const ids = res?.playlist?.trackIds?.map((v) => v.id) ?? [];
				const songs = await getSongDetail(ids);
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
				<LazyImage
					alt="播放列表图片"
					className="playlist-cover-img"
					src={playlist?.playlist?.coverImgUrl || ""}
				/>
				<div className="playlist-info">
					<div className="playlist-name">{playlist?.playlist?.name || ""}</div>
					{playlistSongs && (
						<div className="playlist-stat">
							{playlistSongs.length} 首歌曲 ·{" "}
							{formatDurationCN(totalDuration || 0)}
						</div>
					)}
					<div className="playlist-creator">
						<LazyImage
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
						<button
							className="playlist-play-btn"
							type="button"
							onClick={async () => {
								function getShuffledArr<T>(arr: T[]): T[] {
									const newArr = arr.slice();
									for (let i = newArr.length - 1; i > 0; i--) {
										const rand = Math.floor(Math.random() * (i + 1));
										[newArr[i], newArr[rand]] = [newArr[rand], newArr[i]];
									}
									return newArr;
								}
								if (playlistSongs) {
									await sendMsgToAudioThread("setPlaylist", {
										songs: getShuffledArr(playlistSongs).map((v, i) => ({
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
							乱序播放歌单
						</button>
					</div>
				</div>
			</div>
			<div className="playlist-songs">
				{playlistSongs ? (
					<PlaylistView songs={playlistSongs} />
				) : (
					<div className="playlist-loading">
						<BarLoader color="white" />
						<div>正在加载歌单...</div>
					</div>
				)}
			</div>
		</div>
	);
};
