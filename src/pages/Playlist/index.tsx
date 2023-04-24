import { useAtomValue } from "jotai";
import { useParams } from "react-router-dom";
import { NCMSongDetail, getSongDetailAtom, ncmAPIAtom } from "../../ncm-api";
import { useEffect, useMemo, useRef, useState } from "react";
import "./index.sass";
import { BarLoader } from "react-spinners";
import { sendMsgToAudioThread } from "../../tauri-api";
import {
	AutoSizer,
	CellMeasurer,
	CellMeasurerCache,
	List,
	ListRowRenderer,
} from "react-virtualized";
import { LazyImage } from "../../components/LazyImage";

const cache = new CellMeasurerCache({
	defaultHeight: 64,
	fixedWidth: true,
});

const formatDurationCN = (duration: number) => {
	const d = Math.floor(duration / 1000);
	const h = Math.floor(d / 3600);
	const m = Math.floor((d % 3600) / 60);
	const s = Math.floor((d % 3600) % 60);
	return h > 0 ? `${h} 时 ${m} 分 ${s} 秒` : `${m} 分 ${s} 秒`;
};

const formatDuration = (duration: number) => {
	const d = Math.floor(duration / 1000);
	const h = Math.floor(d / 3600);
	const m = Math.floor((d % 3600) / 60);
	const s = Math.floor((d % 3600) % 60);
	return h > 0
		? `${h}:${"0".repeat(2 - m.toString().length)}${m}:${"0".repeat(
				2 - s.toString().length,
		  )}${s}`
		: `${m}:${"0".repeat(2 - s.toString().length)}${s}`;
};

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

	const rowRender: ListRowRenderer = ({ index, key, style, parent }) => {
		const v = playlistSongs!![index];
		return (
			<CellMeasurer
				cache={cache}
				key={key}
				rowIndex={index}
				overscanRowCount={10}
				parent={parent}
			>
				<div
					className={`song-btn ${index % 2 ? "odd" : "even"}`}
					key={`playlist-songs-${v.id}`}
					style={style}
					onDoubleClick={async () => {
						if (playlistSongs) {
							await sendMsgToAudioThread("setPlaylist", {
								songs: playlistSongs.map((v, i) => ({
									ncmId: String(v.id),
									localFile: "",
									duration: 0,
									origOrder: i,
								})),
							});
							await sendMsgToAudioThread("jumpToSong", {
								songIndex: index,
							});
						}
					}}
				>
					<LazyImage
						width={32}
						height={32}
						alt={`歌曲 ${v.name} 的专辑图片`}
						className="song-album-img"
						src={
							v?.al?.picUrl
								? `${v.al.picUrl}?imageView&enlarge=1&thumbnail=${
										32 * window.devicePixelRatio
								  }y${32 * window.devicePixelRatio}`
								: ""
						}
					/>
					<div>{v.name}</div>
					<div style={{ flex: "1" }} />
					<div className="duration">{formatDuration(v.dt)}</div>
				</div>
			</CellMeasurer>
		);
	};

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
					<AutoSizer>
						{({ width, height }) => (
							<List
								width={width}
								height={height}
								overscanRowCount={16}
								rowCount={playlistSongs.length}
								rowHeight={64}
								rowRenderer={rowRender}
							/>
						)}
					</AutoSizer>
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
