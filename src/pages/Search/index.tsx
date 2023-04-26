import { atom, useAtom, useAtomValue } from "jotai";
import { ncmAPIAtom } from "../../ncm-api";
import { useCallback, useLayoutEffect, useState } from "react";
import { useSearchParams } from "react-router-dom";
import "./index.sass";

const searchKeywordAtom = atom("");
const searchResultAtom = atom([]);

// const searchResultAtom = atom(async (get) => {
// 	const api = await get(ncmAPIAtom);
// 	const keyword = get(searchKeywordAtom);
// 	const data = await api.request(
// 		"https://interface.music.163.com/eapi/cloudsearch/pc",
// 		{
// 			s: keyword,
// 			type: 1,
// 			limit: 30,
// 			offset: 0,
// 			total: true,
// 		},
// 	);
// 	console.log(data);
// 	return data;
// });

export const SearchPage: React.FC = () => {
	const [params] = useSearchParams();
	const keyword = params.get("keyword") ?? "";
	const [searchResult, setSearchResult] = useState([]);

	const searchForKeyword = useCallback(
		async (keyword: string) => {},
		[keyword],
	);

	useLayoutEffect(() => {
		setSearchResult([]);
	}, [keyword]);

	return (
		<div className="search-page">
			<div>“{keyword}” 的搜索结果</div>
		</div>
	);
};
