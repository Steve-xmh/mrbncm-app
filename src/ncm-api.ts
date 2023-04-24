import { Body, Client, ResponseType, getClient } from "@tauri-apps/api/http";
import { Cookie, ncmCookieAtom } from "./ncm-cookie";
import {
	eapiDecrypt,
	eapiEncryptForRequest,
	sendMsgToAudioThread,
} from "./tauri-api";
import { atom } from "jotai";
import { invoke } from "@tauri-apps/api/tauri";

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

const songsCache = new Map<number, NCMSongDetail>();

export const getSongDetailAtom = atom(async (get) => {
	const ncm = await get(ncmAPIAtom);
	return async (ids: number[]) => {
		const results = new Map<number, NCMSongDetail>();
		const uncachedIds = ids.filter((id) => {
			const c = songsCache.get(id);
			if (c) {
				results.set(id, c);
				return false;
			} else {
				return true;
			}
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
							songsCache.set(song.id, song);
						}
					}),
			);
		}
		await Promise.all(songsThreads);
		return ids.map((v) => results.get(v)!!);
	};
});
