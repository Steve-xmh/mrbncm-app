import { RouteObject, createHashRouter } from "react-router-dom";
import { LoginPage } from "./Login";
import { SettingsPage } from "./Settings";
import { MainPage } from "./Main";
import { ErrorPage } from "./Error";
import { PlaylistPage } from "./Playlist";
import App from "../App";
import { RefObject, createRef } from "react";
import { SearchPage } from "./Search";

type RouteObjectWithRef = Omit<RouteObject, "children"> & {
	nodeRef: RefObject<HTMLDivElement>;
	children?: RouteObjectWithRef[];
};

export const routes: RouteObjectWithRef[] = [
	{
		path: "/",
		element: <App />,
		errorElement: <ErrorPage />,
		nodeRef: createRef(),
		children: [
			{
				path: "main",
				element: <MainPage />,
				errorElement: <ErrorPage />,
				nodeRef: createRef(),
			},
			{
				path: "login",
				element: <LoginPage />,
				errorElement: <ErrorPage />,
				nodeRef: createRef(),
			},
			{
				path: "settings",
				element: <SettingsPage />,
				errorElement: <ErrorPage />,
				nodeRef: createRef(),
			},
			{
				path: "playlist/:id",
				element: <PlaylistPage />,
				errorElement: <ErrorPage />,
				nodeRef: createRef(),
			},
			{
				path: "search",
				element: <SearchPage />,
				errorElement: <ErrorPage />,
				nodeRef: createRef(),
			},
			{
				path: "*",
				element: <ErrorPage />,
				errorElement: <ErrorPage />,
				nodeRef: createRef(),
			},
		],
	},
];

export const router = createHashRouter(routes as RouteObject[]);
