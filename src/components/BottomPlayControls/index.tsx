import { useEffect, useState } from "react";
import {
	invokeSyncStatus,
	listenAudioThreadEvent,
	sendMsgToAudioThread,
} from "../../tauri-api";
import { NCMSongDetail, getSongDetailAtom } from "../../ncm-api";
import { useAtomValue } from "jotai";
import IconRewind from "../../assets/icon_rewind.svg?url";
import IconForward from "../../assets/icon_forward.svg?url";
import IconPlay from "../../assets/icon_play.svg?url";
import { TextMarquee } from "../TextMarquee";
import "./index.sass";

export const BottomPlayControls: React.FC = () => {
	const getSongDetail = useAtomValue(getSongDetailAtom);
	const [ncmID, setNCMID] = useState("");
	const [playPos, setPlayPos] = useState(0);
	const [duration, setDuration] = useState(0);
	const [isPlaying, setPlaying] = useState(false);
	const [songInfo, setSongInfo] = useState<NCMSongDetail | null>(null);

	useEffect(() => {
		let canceled = false;
		const unlisten = listenAudioThreadEvent((evt) => {
			if (evt.payload.type === "syncStatus") {
				console.log(evt);
				setNCMID(evt.payload.data.ncmId);
				setDuration(evt.payload.data.duration);
				setPlayPos(evt.payload.data.position);
				setPlaying(evt.payload.data.isPlaying);
			} else if (evt.payload.type === "playPosition") {
				setPlayPos(evt.payload.data.position);
			} else if (evt.payload.type === "loadAudio") {
				console.log(evt);
				setNCMID(evt.payload.data.ncmId);
				setDuration(evt.payload.data.duration);
			} else if (evt.payload.type === "loadingAudio") {
				console.log(evt);
				setNCMID(evt.payload.data.ncmId);
			}
		}).then((v) => {
			invokeSyncStatus();
			if (canceled) v();
			return v;
		});

		return () => {
			canceled = true;
			unlisten.then((v) => v()); // TODO: 确认不会泄露
		};
	}, []);

	useEffect(() => {
		console.log(ncmID);
		if (Number.isNaN(parseInt(ncmID))) return;
		let canceled = false;

		(async () => {
			const [songDetail] = await getSongDetail([parseInt(ncmID)]);
			console.log(songDetail);
			if (!canceled) setSongInfo(songDetail);
		})();

		return () => {
			canceled = true;
		};
	}, [ncmID]);

	return (
		<div className="playbar">
			<div className="playing-song">
				<img
					width={64}
					height={64}
					alt="专辑图片"
					src={songInfo?.al?.picUrl || ""}
				/>
				<div className="song-info">
					<TextMarquee style={{ whiteSpace: "nowrap" }}>{songInfo?.name || ""}</TextMarquee>
					<TextMarquee style={{ whiteSpace: "nowrap" }}>
						{songInfo?.ar?.map((v) => v.name).join(" - ")}
					</TextMarquee>
				</div>
			</div>
			<div className="play-controls">
				<button
					onClick={() => {
						sendMsgToAudioThread("prevSong");
					}}
				>
					<img alt="上一首歌曲" src={IconRewind} />
				</button>
				<button
					onClick={() => {
						sendMsgToAudioThread("prevSong");
					}}
				>
					<img alt="播放/暂停" src={IconPlay} />
				</button>
				<button
					onClick={() => {
						sendMsgToAudioThread("nextSong");
					}}
				>
					<img alt="下一首歌曲" src={IconForward} />
				</button>
			</div>
			<div className="side-buttons"></div>
		</div>
	);
};
