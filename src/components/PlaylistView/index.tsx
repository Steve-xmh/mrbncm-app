import type { NCMSongDetail } from "../../ncm-api";
import "./index.sass";
import { sendMsgToAudioThread } from "../../tauri-api";

import {
	AutoSizer as _AutoSizer,
	List as _List,
	InfiniteLoader as _InfiniteLoader,
	CellMeasurer as _CellMeasurer,
	CellMeasurerProps,
	ListRowRenderer,
	ListProps,
	AutoSizerProps,
	CellMeasurerCache,
} from "react-virtualized";

const List = _List as unknown as React.FC<ListProps> & _List;

const AutoSizer = _AutoSizer as unknown as React.FC<AutoSizerProps> &
	_AutoSizer;
const CellMeasurer = _CellMeasurer as unknown as React.FC<CellMeasurerProps> &
	_CellMeasurer;

import { LazyImage } from "../../components/LazyImage";
import { formatDuration } from "../../utils/format";

const cache = new CellMeasurerCache({
	defaultHeight: 64,
	fixedWidth: true,
});

export const PlaylistView: React.FC<{
	songs: NCMSongDetail[];
}> = (props) => {
	const rowRender: ListRowRenderer = ({ index, key, style, parent }) => {
		const v = props.songs[index];
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
						if (props.songs) {
							await sendMsgToAudioThread("setPlaylist", {
								songs: props.songs.map((v, i) => ({
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
		<AutoSizer>
			{({ width, height }) => (
				<List
					width={width}
					height={height}
					overscanRowCount={16}
					rowCount={props.songs.length}
					rowHeight={64}
					rowRenderer={rowRender}
				/>
			)}
		</AutoSizer>
	);
};
