import { useAtomValue } from "jotai";
import { ncmCookieAtom } from "../../ncm-cookie";
import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import "./index.sass";
import { ncmAPIAtom, userPlaylistAtom } from "../../ncm-api";

export const MainPage: React.FC = () => {
	return <div>欢迎来到 MRBNCM App！</div>;
};
