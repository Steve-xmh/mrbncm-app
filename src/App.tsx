import {
	createHashRouter,
	RouterProvider,
	NavigateFunction,
} from "react-router-dom";
import { LoginPage } from "./pages/Login";
import { MainPage } from "./pages/Main";
import { Icon } from "@iconify/react";
import settingIcon from "@iconify/icons-uil/setting";
import playlistIcon from "@iconify/icons-mdi/playlist-music";
import baselineHome from "@iconify/icons-ic/baseline-home";
import outlineExpandMore from "@iconify/icons-ic/outline-expand-more";
import outlineExpandLess from "@iconify/icons-ic/outline-expand-less";
import { ErrorPage } from "./pages/Error";
import { SettingsPage } from "./pages/Settings";
import { Suspense, useRef, useState } from "react";
import { useAtom, useAtomValue } from "jotai";
import { userInfoAtom, userPlaylistAtom } from "./ncm-api";
import { PlaylistPage } from "./pages/Playlist";
import { BarLoader } from "react-spinners";
import { BottomPlayControls } from "./components/BottomPlayControls";
import { LazyImage } from "./components/LazyImage";
import { atomWithStorage } from "jotai/utils";

let navigate: NavigateFunction = (path) => {
	location.hash = `#${path}`;
};

const router = createHashRouter([
	{
		path: "/login",
		element: <LoginPage />,
	},
	{
		path: "/settings",
		element: <SettingsPage />,
	},
	{
		path: "/",
		element: <MainPage />,
		errorElement: <ErrorPage />,
	},
	{
		path: "/playlist/:id",
		element: <PlaylistPage />,
	},
]);

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
	const [sidebarWidth, setSidebarWidth] = useAtom(sidebarWidthAtom)
	
	const onSidebarDraggerMouseDown = () => {
		const onMouseMove = (evt: MouseEvent) => {
			setSidebarWidth(Math.max(192, Math.min(512, evt.clientX)))
		}
		const onMouseUp = () => {
			window.removeEventListener("mousemove", onMouseMove)
			window.removeEventListener("mouseup", onMouseUp)
		}
		window.addEventListener("mousemove", onMouseMove)
		window.addEventListener("mouseup", onMouseUp)
	}

	return (
		<div className="container">
			<div className="upper-container">
				<div className="sidebar" ref={sidebarRef} style={{
					width:`${sidebarWidth}px`
				}}>
					<input className="search-input" placeholder="搜索……" />
					<button
						className="sidebar-btn"
						onClick={() => {
							navigate("/");
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
				<div className="dragger" style={{
					cursor: sidebarWidth === 192 ? "e-resize" : sidebarWidth === 512 ? "w-resize" : "ew-resize",
				}} onMouseDown={onSidebarDraggerMouseDown} />
				<div className="main-page-router">
					<Suspense fallback={<BarLoader />}>
						<RouterProvider router={router} />
					</Suspense>
				</div>
			</div>
			<BottomPlayControls />
		</div>
	);
}

export default App;
