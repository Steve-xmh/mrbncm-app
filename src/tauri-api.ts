import { invoke } from "@tauri-apps/api/tauri";
import { listen, EventCallback } from "@tauri-apps/api/event";
import { uid } from "uid";

const msgTasks = new Map<string, (value: any) => void>();

invoke("init_audio_thread");
listen<{
	callbackId: string;
	data: any;
}>("on_audio_thread_message", (evt) => {
	const resolve = msgTasks.get(evt.payload.callbackId);
	if (resolve) {
		msgTasks.delete(evt.payload.callbackId);
		resolve(evt.payload.data);
	}
});

export interface AudioThreadMessage {
	type: string;
	data: any;
}

export const invokeSyncStatus = () => invoke("init_audio_thread");
export const listenAudioThreadEvent = (
	handler: EventCallback<AudioThreadMessage>,
) => listen("on-audio-thread-event", handler);

export function sendMsgToAudioThread(
	msgType: string,
	data: any = {},
): Promise<any> {
	const id = uid(32) + Date.now();
	return new Promise((resolve) => {
		msgTasks.set(id, resolve);
		invoke("send_msg_to_audio_thread", {
			msg: {
				[msgType]: {
					callbackId: id,
					...data,
				},
			},
		});
	});
}

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
