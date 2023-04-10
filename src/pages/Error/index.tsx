import { useRouteError } from "react-router-dom";
import "./index.sass";

export const ErrorPage: React.FC = () => {
	const error = useRouteError() as any;
	console.error(error);

	return (
		<div className="error-page">
			<div>
				<h1>出错啦</h1>
				<p>{error.statusText || error.message || error}</p>
			</div>
		</div>
	);
};
