import { atom, useAtomValue } from "jotai";
import { Suspense } from "react";
import "./index.sass";
import { ncmAPIAtom } from "../../ncm-api";
import { LazyImage } from "../../components/LazyImage";
import { useNavigate } from "react-router-dom";
import daliyIcon from "../../assets/daily.svg?url";

const recommendResourceAtom = atom(async (get) => {
	const api = await get(ncmAPIAtom);
	const data = await api.request(
		"https://music.163.com/eapi/v1/discovery/recommend/resource",
	);
	return data;
});

export const HeadCard: React.FC<
	React.ButtonHTMLAttributes<HTMLButtonElement> & {
		src: string;
		label: string;
	}
> = (props) => {
	const { src, label, ...others } = props;
	return (
		<button className="head-card" {...others}>
			<LazyImage src={src} />
			<div>{label}</div>
		</button>
	);
};

export const MainPage: React.FC = () => {
	const recommendResource = useAtomValue(recommendResourceAtom);
	const navigate = useNavigate();
	return (
		<div className="main-page">
			<h2>欢迎来到 MRBNCM App！</h2>
			<h5>推荐歌单</h5>
			<div className="recommend">
				<div>
					<button className="head-card">
						<div className="daily-text">{new Date().getDate()}</div>
						<div className="daily-icon">
							<svg
								viewBox="0 0 80 80"
								id="calendar_box"
								xmlns="http://www.w3.org/2000/svg"
							>
								<path
									d="M-227.5-862.5h-47a1.5 1.5 0 0 0 0 3h47a1.5 1.5 0 0 0 0-3z"
									fill="#fff"
								/>
								<path
									d="M-229-874.5h-3v-2.5a1.5 1.5 0 0 0-3 0v2.5h-32v-2.5a1.5 1.5 0 0 0-3 0v2.5h-3c-6.627 0-12 5.373-12 12v37c0 6.627 5.373 12 12 12h44c6.627 0 12-5.373 12-12v-37c0-6.627-5.373-12-12-12zm9 49c0 4.963-4.037 9-9 9h-44c-4.963 0-9-4.037-9-9v-37c0-4.963 4.037-9 9-9h3v2.5a1.5 1.5 0 0 0 3 0v-2.5h32v2.5a1.5 1.5 0 0 0 3 0v-2.5h3c4.963 0 9 4.037 9 9z"
									fill="#fff"
								/>
								<path d="M-583-1291H597v2560H-583z" fill="none" />
								<path
									d="M-273-874.5h44c6.627 0 12 5.373 12 12v37c0 6.627-5.373 12-12 12h-44c-6.627 0-12-5.373-12-12v-37c0-6.627 5.373-12 12-12z"
									fill="none"
								/>
								<g fill="#fff">
									<path d="M63.5 23.5h-47a1.5 1.5 0 0 0 0 3h47a1.5 1.5 0 0 0 0-3z" />
									<path d="M62 11.5h-3V9a1.5 1.5 0 0 0-3 0v2.5H24V9a1.5 1.5 0 0 0-3 0v2.5h-3c-6.627 0-12 5.373-12 12v37c0 6.627 5.373 12 12 12h44c6.627 0 12-5.373 12-12v-37c0-6.627-5.373-12-12-12zm9 49c0 4.963-4.037 9-9 9H18c-4.963 0-9-4.037-9-9v-37c0-4.963 4.037-9 9-9h3V17a1.5 1.5 0 0 0 3 0v-2.5h32V17a1.5 1.5 0 0 0 3 0v-2.5h3c4.963 0 9 4.037 9 9z" />
								</g>
							</svg>
						</div>
						<div>每日精选歌单</div>
					</button>
				</div>
				<Suspense>
					{recommendResource?.recommend?.map((v, i) => (
						<div key={`rcmd-card-${i}`}>
							<HeadCard
								onClick={() => navigate(`/playlist/${v.id}`)}
								src={v.picUrl}
								label={v.name}
							/>
						</div>
					))}
				</Suspense>
			</div>
		</div>
	);
};
