import { Body, Client, ResponseType, getClient } from "@tauri-apps/api/http";
import { Cookie } from "./ncm-cookie";
import { eapiDecrypt, eapiEncryptForRequest } from "./tauri-api";

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

	async request<T>(url: string, data: string): Promise<T> {
		const client = await this.getClient();
		const urlObj = new URL(url);
		if (urlObj.pathname.startsWith("/eapi")) {
			const cookies = this.cookies
				.map((v) => `${v.Name}=${v.Value}`)
				.join(", ");
			console.log(cookies);
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
						Cookies: cookies,
					},
				},
			);
			const hex = JSON.parse(await eapiDecrypt(binaryToHex(res.data)));
			if (res.ok) {
				return hex;
			} else {
				throw hex;
			}
		} else {
			const res = await client.post<T>(url, Body.text(data));
			return res.data;
		}
	}
}
