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

	async request<T = any>(
		url: string,
		data: string,
		resType: ResponseType = ResponseType.Binary,
	): Promise<T> {
		const client = await this.getClient();
		const urlObj = new URL(url);
		const cookies = this.cookies.map((v) => `${v.Name}=${v.Value}`).join("; ");
		// console.log(
		// 	url,
		// 	data,
		// 	this.cookies.map((v) => `${v.Name}=${v.Value}`),
		// );
		if (urlObj.pathname.startsWith("/eapi")) {
			const res = await client.post<number[]>(
				url,
				Body.form({
					params: await eapiEncryptForRequest(
						urlObj.pathname.replace("/eapi", "/api"),
						data,
					),
				}),
				{
					responseType: ResponseType.Binary,
					headers: {
						cookie: cookies,
						origin: "orpheus://orpheus",
						"user-agent":
							"Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36 Chrome/91.0.4472.164 NeteaseMusicDesktop/2.10.7.200791",
					},
				},
			);
			// console.log(res);
			if (res.ok) {
				if (res.data[0] === 123) {
					// 尝试直接解码，可能是明文
					try {
						const decoder = new TextDecoder();
						const decoded = JSON.parse(
							decoder.decode(new Uint8Array(res.data)),
						);
						console.log(decoded);
						return decoded;
					} catch {}
				}
				const de = await eapiDecrypt(binaryToHex(res.data));
				const hex = JSON.parse(de);
				return hex;
			} else {
				if (res.data[0] === 123) {
					// 尝试直接解码，可能是明文
					try {
						const decoder = new TextDecoder();
						throw JSON.parse(decoder.decode(new Uint8Array(res.data)));
					} catch {}
				}
				const de = await eapiDecrypt(binaryToHex(res.data));
				const hex = JSON.parse(de);
				throw hex;
			}
		} else {
			const res = await client.post<T>(url, Body.text(data), {
				responseType: resType,
				headers: {
					cookie: cookies,
					origin: "orpheus://orpheus",
					"user-agent":
						"Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36 Chrome/91.0.4472.164 NeteaseMusicDesktop/2.10.7.200791",
				},
			});
			return res.data;
		}
	}
}

export const ncmAPIAtom = atom((get) => {
	const cookies = get(ncmCookieAtom);
	sendMsgToAudioThread("setCookie", {
		cookie: cookies.map((v) => `${v.Name}=${v.Value}`).join("; "),
	});
	return new NCMAPI(cookies);
});

export const userInfoAtom = atom(async (get) => {
	const api = get(ncmAPIAtom);
	return await api.request("https://music.163.com/api/nuser/account/get", "{}");
});

export const userSubCountAtom = atom(async (get) => {
	const api = get(ncmAPIAtom);
	return await api.request("https://music.163.com/api/subcount", "{}");
});

export const userPlaylistAtom = atom(async (get) => {
	const api = get(ncmAPIAtom);
	const userInfo = await get(userInfoAtom);
	return await api.request(
		"https://music.163.com/eapi/user/playlist",
		JSON.stringify({
			uid: userInfo?.account?.id || 0,
			limit: 30,
			offset: 0,
			includeVideo: true,
		}),
	);
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

export const getSongDetailAtom = atom((get) => {
	const ncm = get(ncmAPIAtom);
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
					.request(
						"https://music.163.com/eapi/v3/song/detail",
						JSON.stringify({
							c: JSON.stringify(postData),
							e_r: true,
						}),
						ResponseType.JSON,
					)
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
