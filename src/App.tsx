import { Outlet, useLocation, useNavigate, useOutlet } from "react-router-dom";
import { Icon } from "@iconify/react";
import settingIcon from "@iconify/icons-uil/setting";
import playlistIcon from "@iconify/icons-mdi/playlist-music";
import baselineHome from "@iconify/icons-ic/baseline-home";
import outlineExpandMore from "@iconify/icons-ic/outline-expand-more";
import outlineExpandLess from "@iconify/icons-ic/outline-expand-less";
import { Suspense, useEffect, useRef, useState } from "react";
import { useAtom, useAtomValue } from "jotai";
import { userInfoAtom, userPlaylistAtom } from "./ncm-api";
import { SwitchTransition, CSSTransition } from "react-transition-group";
import { BottomPlayControls } from "./components/BottomPlayControls";
import { LazyImage } from "./components/LazyImage";
import { atomWithStorage } from "jotai/utils";
import { getCurrent } from "@tauri-apps/api/window";
import { routes } from "./pages";
import { AMLLWrapper } from "./components/AMLLWrapper";

const UserInfoButton: React.FC = () => {
	const userInfo = useAtomValue(userInfoAtom);

	return (
		<button className="sidebar-btn" onClick={() => {}}>
			{userInfo?.profile?.avatarUrl ? (
				<LazyImage
					width={32}
					height={32}
					alt="头像"
					className="avatar"
					src={userInfo?.profile?.avatarUrl || ""}
				/>
			) : (
				<div className="no-avatar" />
			)}
			{userInfo?.profile?.nickname || "未登录"}
		</button>
	);
};

const UserPlaylists: React.FC = () => {
	const playlists = useAtomValue(userPlaylistAtom)?.playlist ?? [];
	const navigate = useNavigate();

	return (
		<>
			{playlists.map((v) => (
				<button
					key={`playlist-btn-${v.id}`}
					className="sidebar-btn"
					onClick={() =>
						navigate(`/playlist/${v.id}?name=${encodeURIComponent(v.name)}`)
					}
				>
					<Icon width={20} icon={playlistIcon} inline className="icon" />
					<div className="name">{v.name}</div>
				</button>
			))}
		</>
	);
};

const sidebarWidthAtom = atomWithStorage("sidebar-width", 256);

function App() {
	const [playlistOpened, setPlaylistOpened] = useState(false);
	const sidebarRef = useRef<HTMLDivElement>(null);
	const [sidebarWidth, setSidebarWidth] = useAtom(sidebarWidthAtom);
	const navigate = useNavigate();
	const location = useLocation();
	const currentOutlet = useOutlet();
	const [lyricPageOpened, setLyricPageOpened] = useState(false);
	const { nodeRef } =
		routes[0].children?.find(
			(route) => `/${route.path}` === location.pathname,
		) ?? {};

	const onSidebarDraggerMouseDown = () => {
		const onMouseMove = (evt: MouseEvent) => {
			setSidebarWidth(Math.max(192, Math.min(512, evt.clientX)));
		};
		const onMouseUp = () => {
			window.removeEventListener("mousemove", onMouseMove);
			window.removeEventListener("mouseup", onMouseUp);
		};
		window.addEventListener("mousemove", onMouseMove);
		window.addEventListener("mouseup", onMouseUp);
	};

	useEffect(() => {
		setTimeout(() => {
			getCurrent().show();
		}, 100);
	}, []);

	return (
		<>
			<div className="container">
				<div className="upper-container">
					<div
						className="sidebar"
						ref={sidebarRef}
						style={{
							width: `${sidebarWidth}px`,
						}}
					>
						<input
							className="search-input"
							placeholder="搜索……"
							onKeyDown={(evt) => {
								if (evt.key === "Enter") {
									navigate(
										`/search?keyword=${(evt.target as HTMLInputElement).value}`,
									);
									(evt.target as HTMLInputElement).blur();
								}
							}}
						/>
						<button
							className="sidebar-btn"
							onClick={() => {
								navigate("/main");
							}}
						>
							<Icon width={20} icon={baselineHome} inline className="icon" />
							主页
						</button>
						<button
							className="sidebar-btn"
							onClick={() => setPlaylistOpened((v) => !v)}
						>
							<Icon
								width={20}
								icon={playlistOpened ? outlineExpandLess : outlineExpandMore}
								inline
								className="icon"
							/>
							{playlistOpened ? "收起歌单列表" : "展开歌单列表"}
						</button>
						{playlistOpened && (
							<Suspense>
								<div
									style={{
										minHeight: 0,
										flex: 1,
										overflowX: "hidden",
										overflowY: "auto",
										display: "flex",
										flexDirection: "column",
									}}
								>
									<UserPlaylists />
								</div>
							</Suspense>
						)}
						{/* <div className="spacer" /> */}
						<Suspense>
							<UserInfoButton />
						</Suspense>
						<button
							className="sidebar-btn"
							onClick={() => {
								navigate("/settings");
							}}
						>
							<Icon width={20} icon={settingIcon} inline className="icon" />
							设置
						</button>
					</div>
					<div
						className="dragger"
						style={{
							cursor:
								sidebarWidth === 192
									? "e-resize"
									: sidebarWidth === 512
									? "w-resize"
									: "ew-resize",
						}}
						onMouseDown={onSidebarDraggerMouseDown}
					/>
					<div className="main-page-router">
						<SwitchTransition>
							<CSSTransition
								key={location.key}
								nodeRef={nodeRef}
								timeout={200}
								classNames="inner-page"
							>
								{() => (
									<div ref={nodeRef}>
										<Suspense>{currentOutlet}</Suspense>
									</div>
								)}
							</CSSTransition>
						</SwitchTransition>
					</div>
				</div>
				<BottomPlayControls />
			</div>
			<div>
				<AMLLWrapper />
			</div>
		</>
	);
}

export default App;
