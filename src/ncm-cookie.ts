import { LargeNumberLike } from "crypto";
import { atom } from "jotai";
import { atomWithStorage } from "jotai/utils";

export interface Cookie {
	Creation: number;
	Domain: string;
	Expires: number;
	HasExpires: number;
	Httponly: number;
	LastAccess: number;
	Name: string;
	Path: string;
	Secure: number;
	Url: string;
	Value: string;
}

const rawNCMCookieAtom = atomWithStorage("ncm-cookie", "[]");
export const ncmCookieAtom = atom<Cookie[]>((get) =>
	JSON.parse(get(rawNCMCookieAtom)),
);
export const setNCMCookieAtom = atom(null, (_get, set, value) => {
	if (value instanceof Array) {
		set(rawNCMCookieAtom, JSON.stringify(value));
	}
});
