import { createBrowserRouter, RouterProvider } from "react-router-dom";
import { LoginPage } from "./pages/Login";
import { MainPage } from "./pages/Main";
import { Icon } from "@iconify/react";
import settingIcon from "@iconify/icons-uil/setting";
import baselineHome from "@iconify/icons-ic/baseline-home";
import { ErrorPage } from "./pages/Error";
import { SettingsPage } from "./pages/Settings";
import { Suspense, useCallback } from "react";
import { useAtom, useAtomValue } from "jotai";
import { userInfoAtom } from "./ncm-api";

const router = createBrowserRouter([
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
]);

const UserInfoButton: React.FC = () => {
	const userInfo = useAtomValue(userInfoAtom);

	return (
		<button className="sidebar-btn" onClick={() => {}}>
			<img
				width={32}
				height={32}
				alt="头像"
				className="avatar"
				src={userInfo.profile.avatarUrl}
			/>
			{userInfo.profile.nickname}
		</button>
	);
};

function App() {
	const navigate = useCallback((path: string) => {
		history.pushState({}, "", path);
		location.reload();
	}, []);

	return (
		<div className="container">
			<div className="upper-container">
				<div className="sidebar">
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
					<div className="spacer" />
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
				<div className="dragger" />
				<div className="main-page-router">
					<RouterProvider router={router} />
				</div>
			</div>
			<div className="playbar">
				<img width={64} height={64} alt="专辑图片" />
			</div>
		</div>
	);
}

export default App;
