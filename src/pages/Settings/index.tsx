import { Link } from "react-router-dom";
import "./index.sass";

export const SettingsPage: React.FC = () => {
	return (
		<div className="settings-page">
			<h2>账户</h2>
			<div className="block grid">
				<div>重新设置 Cookie</div>
				<Link className="btn" to="/login?reset=true">
					重新赋予 Cookie
				</Link>
			</div>
			<h2>关于</h2>
			<div className="block">
				<div>MRBNCM App</div>
				<div style={{ fontSize: "11px" }}>By SteveXMH</div>
			</div>
		</div>
	);
};
