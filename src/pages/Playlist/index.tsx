import { useAtomValue } from "jotai";
import { useParams } from "react-router-dom";
import { NCMSongDetail, getSongDetailAtom, ncmAPIAtom } from "../../ncm-api";
import { useEffect, useRef, useState } from "react";
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

const cache = new CellMeasurerCache({
	defaultHeight: 64,
	fixedWidth: true,
});

export const PlaylistPage: React.FC = () => {
	const ncm = useAtomValue(ncmAPIAtom);
	const getSongDetail = useAtomValue(getSongDetailAtom);
	const [playlist, setPlaylist] = useState({});
	const [playlistSongs, setPlaylistSongs] = useState<NCMSongDetail[] | null>(
		null,
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
					<img
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
						loading="lazy"
					/>
					<div>{v.name}</div>
				</div>
			</CellMeasurer>
		);
	};

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
					<AutoSizer>
						{({ width, height }) => (
							<List
								width={width}
								height={height}
								rowCount={playlistSongs.length}
								rowHeight={64}
								rowRenderer={rowRender}
							/>
						)}
					</AutoSizer>
				) : (
					<BarLoader color="white" />
				)}
			</div>
		</div>
	);
};
