import { convertFileSrc } from "@tauri-apps/api/core";

export function debounce(fn: (...args: any[]) => void, delay: number) {
    let timer: number | null = null;
    return (...args: any[]) => {
        if (timer) {
            clearTimeout(timer);
        }
        timer = setTimeout(() => fn(...args), delay);
    };
}

export function UrlEncode(url: string) {
    return encodeURIComponent(url);
}

export function UrlDecode(url: string) {
    return decodeURIComponent(url);
}

export function toImgSrc(path: string): string {
    // Windows 路径转为 URL 友好的正斜杠
    const normalized = path.replace(/\\/g, '/')
    return convertFileSrc(normalized)
}
