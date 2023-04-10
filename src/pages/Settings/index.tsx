import { Link, useRouteError } from "react-router-dom";

export const SettingsPage: React.FC = () => {
	return (
		<div className="settings-page">
			<div>
				<Link to="/login?reset=true">重新赋予 Cookie</Link>
			</div>
		</div>
	);
};
