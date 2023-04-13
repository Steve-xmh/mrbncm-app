import ReactSlider from "react-slider";
import type { ReactSliderProps } from "react-slider";
import "./index.sass";

export const NowPlayingSlider: React.FC<ReactSliderProps> = (props) => {
	const { className, ...others } = props;
	return (
		<ReactSlider
			className={`now-playing-slider ${className || ""}`}
			{...others}
		/>
	);
};
