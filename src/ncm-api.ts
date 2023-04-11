import { Body, Client, ResponseType, getClient } from "@tauri-apps/api/http";
import { Cookie, ncmCookieAtom } from "./ncm-cookie";
import { eapiDecrypt, eapiEncryptForRequest } from "./tauri-api";
import { atom } from "jotai";

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

	async request<T = any>(url: string, data: string): Promise<T> {
		const client = await this.getClient();
		const urlObj = new URL(url);
		const cookies = this.cookies.map((v) => `${v.Name}=${v.Value}`).join("; ");
		console.log(
			url,
			data,
			this.cookies.map((v) => `${v.Name}=${v.Value}`),
		);
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
			console.log(res);
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
				console.log(hex);
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
				responseType: ResponseType.JSON,
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
			uid: userInfo.account.id,
			limit: 30,
			offset: 0,
			includeVideo: true,
		}),
	);
});
