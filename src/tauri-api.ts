import { invoke } from "@tauri-apps/api/tauri";

export function eapiEncryptForRequest(
	urlPath: string,
	data: string,
): Promise<string> {
	return invoke("tauri_eapi_encrypt_for_request", {
		url: urlPath,
		data,
	});
}

export function eapiEncrypt(data: string): Promise<string> {
	return invoke("tauri_eapi_encrypt", {
		data,
	});
}

export function eapiDecrypt(data: string): Promise<string> {
	return invoke("tauri_eapi_decrypt", {
		data,
	});
}
