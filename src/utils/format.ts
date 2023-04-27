export const formatDurationCN = (duration: number) => {
	const d = Math.floor(duration / 1000);
	const h = Math.floor(d / 3600);
	const m = Math.floor((d % 3600) / 60);
	const s = Math.floor((d % 3600) % 60);
	return h > 0 ? `${h} 时 ${m} 分 ${s} 秒` : `${m} 分 ${s} 秒`;
};

export const formatDuration = (duration: number) => {
	const d = Math.floor(duration / 1000);
	const h = Math.floor(d / 3600);
	const m = Math.floor((d % 3600) / 60);
	const s = Math.floor((d % 3600) % 60);
	return h > 0
		? `${h}:${"0".repeat(2 - m.toString().length)}${m}:${"0".repeat(
				2 - s.toString().length,
		  )}${s}`
		: `${m}:${"0".repeat(2 - s.toString().length)}${s}`;
};
