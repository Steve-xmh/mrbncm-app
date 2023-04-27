import { Body, Client, ResponseType, getClient } from "@tauri-apps/api/http";
import { Cookie, ncmCookieAtom } from "./ncm-cookie";
import {
	eapiDecrypt,
	eapiEncryptForRequest,
	sendMsgToAudioThread,
} from "./tauri-api";
import { atom } from "jotai";
import { invoke } from "@tauri-apps/api/tauri";
import { appWindow } from "@tauri-apps/api/window";
import { appDataDir, appCacheDir } from "@tauri-apps/api/path";
import Database from "tauri-plugin-sql-api";

let client: Client;

function binaryToHex(bins: number[]): string {
	let result = "";
	for (const bin of bins) {
		const h = bin.toString(16);
		if (h.length < 2) {
			result += "0".repeat(2 - h.length);
		}
		result += h;
	}
	return result;
}

export class NCMAPI {
	constructor(private cookies: Cookie[] = []) {
		if (this.cookies.length === 0) {
			this.cookies = JSON.parse(
				JSON.parse(localStorage.getItem("ncm-cookie") ?? '"[]"'),
			);
		}
	}

	private async getClient() {
		if (!client) {
			client = await getClient();
		}
		return client;
	}

	// rome-ignore lint/suspicious/noExplicitAny: <explanation>
	async request<T = any>(
		url: string,
		// rome-ignore lint/suspicious/noExplicitAny: <explanation>
		data: any = {},
	): Promise<T> {
		return invoke("tauri_eapi_request", {
			url,
			data,
		});
	}
}

export const ncmAPIAtom = atom(async (get) => {
	const cookies = get(ncmCookieAtom);
	await sendMsgToAudioThread("setCookie", {
		cookie: cookies.map((v) => `${v.Name}=${v.Value}`).join("; "),
	});
	const api = new NCMAPI(cookies);
	return api;
});

export const userInfoAtom = atom(async (get) => {
	const api = await get(ncmAPIAtom);
	return await api.request("https://music.163.com/api/nuser/account/get");
});

export const userSubCountAtom = atom(async (get) => {
	const api = await get(ncmAPIAtom);
	return await api.request("https://music.163.com/api/subcount");
});

export const userPlaylistAtom = atom(async (get) => {
	const api = await get(ncmAPIAtom);
	const userInfo = await get(userInfoAtom);
	const res = await api.request("https://music.163.com/eapi/user/playlist", {
		uid: userInfo?.account?.id || 0,
		limit: 30,
		offset: 0,
		includeVideo: true,
	});
	return res;
});

export interface NCMSongDetail {
	name: string;
	id: number;
	ar: {
		id: number;
		name: string;
	}[];
	al: {
		id: number;
		name: string;
		picUrl: string;
		tns: string[];
	};
	dt: number;
	alia: string[];
	tns: string[];
}

let database: Database;

async function initSongsCache() {
	const songsCacheDBPath = `sqlite:${await appCacheDir()}songs-cache.db`;
	database = await Database.load(songsCacheDBPath);
	console.log("已初始化歌曲数据库", database.path);
	console.log(
		await database.execute(
			"CREATE TABLE IF NOT EXISTS SONGS_CACHE" +
				"(NCMID UNSIGNED BIT INT PRIMARY KEY NOT NULL UNIQUE," +
				"SONG_DATA TEXT NOT NULL," +
				"EXPIRE_TIME INT NOT NULL)",
		),
	);
	const curDate = Date.now();
	console.log(
		await database.execute("DELETE FROM SONGS_CACHE WHERE EXPIRE_TIME < ?", [
			curDate,
		]),
	);
	appWindow.onCloseRequested(async () => {
		await database.close();
	});
}
initSongsCache();

async function saveSongsCache(songs: NCMSongDetail[]) {
	const expireDate = Date.now() + 30 * 24 * 60 * 60 * 1000;
	await Promise.all(
		songs.map(async (v) => {
			const data = JSON.stringify(v);
			if (!v.id) return;
			await database.execute(
				"REPLACE INTO SONGS_CACHE" +
					"(NCMID, SONG_DATA, EXPIRE_TIME) VALUES " +
					"($1, $2, $3) ",
				[v.id, data, expireDate],
			);
		}),
	);
}

async function searchForSongsCache(
	songIds: number[],
): Promise<NCMSongDetail[]> {
	const curTime = Date.now();
	return (
		await Promise.all(
			songIds.map(async (id) => {
				const c: NCMSongDetail[] = await database.select(
					"SELECT * FROM SONGS_CACHE WHERE NCMID = $1 AND EXPIRE_TIME >= $2 LIMIT 1",
					[id, curTime],
				);
				if (c.length > 0) {
					return c[0];
				}
				return undefined;
			}),
		)
	).filter((v) => v) as NCMSongDetail[];
}

export const getSongDetailAtom = atom(async (get) => {
	const ncm = await get(ncmAPIAtom);
	return async (ids: number[]) => {
		const results = new Map<number, NCMSongDetail>(
			(await searchForSongsCache(ids)).map((v) => [v.id, v]),
		);
		const uncachedIds = ids.filter((id) => {
			return !results.has(id);
		});
		const songsThreads = [];
		for (let i = 0; i < uncachedIds.length; i += 1000) {
			const postData = [];
			for (let j = 0; j < Math.min(1000, uncachedIds.length - i); j++) {
				postData.push({
					id: uncachedIds[i + j],
					v: 0,
				});
			}
			songsThreads.push(
				ncm
					.request("https://music.163.com/eapi/v3/song/detail", {
						c: JSON.stringify(postData),
						e_r: true,
					})
					.then((v) => {
						for (const song of v.songs) {
							results.set(song.id, song);
						}
					}),
			);
		}
		await Promise.all(songsThreads);
		saveSongsCache([...results.values()]).catch((err) => {
			console.warn("缓存歌曲信息出错", err);
		});
		return ids.map((v) => results.get(v)!!);
	};
});
