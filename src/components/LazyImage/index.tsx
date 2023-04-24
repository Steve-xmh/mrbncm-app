import { useEffect, useLayoutEffect, useRef } from "react";

export const LazyImage: React.FC<
	React.PropsWithoutRef<React.HTMLProps<HTMLImageElement>>
> = (props) => {
	const { src, ...other } = props;

	const imgRef = useRef<HTMLImageElement>(null);

	useLayoutEffect(() => {
		const img = imgRef.current;
		if (img) {
			let canceled = false;
			img.style.opacity = "0";
			const firstLoadTime = Date.now();
			const onLoad = async () => {
				if (Date.now() - firstLoadTime > 200) {
					await img.animate(
						[
							{
								opacity: "1",
							},
						],
						{
							duration: 200,
						},
					).finished;
					if (canceled) return;
				}
				img.style.opacity = "1";
			};
			img.addEventListener("load", onLoad);
			return () => {
				img.removeEventListener("load", onLoad);
				canceled = true;
			};
		}
	}, []);

	useEffect(() => {
		const img = imgRef.current;
		if (img && src !== undefined) {
			let canceled = false;
			(async () => {
				await img.animate(
					[
						{
							opacity: "0",
						},
					],
					{
						duration: 200,
					},
				).finished;
				if (canceled) return;
				img.style.opacity = "0";
				img.src = src;
			})();
			return () => {
				canceled = true;
			};
		}
	}, [src]);

	// rome-ignore lint/a11y/useAltText: <explanation>
	return <img ref={imgRef} {...other} />;
};
