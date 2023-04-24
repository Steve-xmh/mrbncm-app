import { atom, useAtomValue } from "jotai";
import { Suspense } from "react";
import "./index.sass";
import { ncmAPIAtom } from "../../ncm-api";
import { LazyImage } from "../../components/LazyImage";

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
	return (
		<div className="main-page">
			<h2>欢迎来到 MRBNCM App！</h2>
			<h5>推荐歌单</h5>
			<div className="recommend">
				<div>
					<HeadCard src="" label="每日精选歌单" />
				</div>
				<Suspense>
					{recommendResource?.recommend?.map((v, i) => (
						<div key={`rcmd-card-${i}`}>
							<HeadCard src={v.picUrl} label={v.name} />
						</div>
					))}
				</Suspense>
			</div>
		</div>
	);
};
