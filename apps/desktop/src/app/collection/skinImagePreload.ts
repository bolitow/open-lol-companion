import {useCallback, useEffect, useRef} from 'react';
import {collectionImageUrl} from './collectionModel';

export interface PreloadImage {
    src: string;
    decoding: string;
    referrerPolicy: string;
    fetchPriority: string;
    addEventListener: (type: 'load' | 'error', listener: () => void) => void;
    removeEventListener: (type: 'load' | 'error', listener: () => void) => void;
    removeAttribute: (name: string) => void;
}
type Entry = {url: string; status: 'queued' | 'loading' | 'ready'; image?: PreloadImage; detach?: () => void};
const MAX_IMAGES = 8, MAX_ACTIVE = 2, INTENT_MS = 150, REQUEST_TIMEOUT_MS = 12000;

/** Un cache de huit splashs maximum, créé uniquement par une intention réelle sur une carte. */
export function createSkinImagePreloader(create: () => PreloadImage = () => new Image()) {
    const entries = new Map<string, Entry>();
    let active = 0, disposed = false, intent: ReturnType<typeof setTimeout> | undefined;
    const clearEntry = (entry: Entry) => {
        entry.detach?.();
        entry.image?.removeAttribute('src');
        if (entry.status === 'loading') active--;
        entries.delete(entry.url);
    };
    const pump = () => {
        if (disposed) return;
        for (const entry of entries.values()) {
            if (active >= MAX_ACTIVE) break;
            if (entry.status !== 'queued') continue;
            const image = create();
            entry.image = image; entry.status = 'loading'; active++;
            const finish = (success: boolean) => {
                if (disposed || entries.get(entry.url) !== entry || entry.status !== 'loading') return;
                entry.detach?.(); active--;
                if (success) entry.status = 'ready';
                else {entry.status = 'queued'; image.removeAttribute('src'); entries.delete(entry.url);}
                pump();
            };
            const ready = () => finish(true), failed = () => finish(false);
            const timeout = setTimeout(failed, REQUEST_TIMEOUT_MS);
            entry.detach = () => {clearTimeout(timeout); image.removeEventListener('load', ready); image.removeEventListener('error', failed);};
            image.addEventListener('load', ready); image.addEventListener('error', failed);
            image.decoding = 'async'; image.referrerPolicy = 'no-referrer'; image.fetchPriority = 'low'; image.src = entry.url;
        }
    };
    const enqueue = (url: string) => {
        if (disposed) return;
        const cached = entries.get(url);
        if (cached) {entries.delete(url); entries.set(url, cached); return;}
        if (entries.size >= MAX_IMAGES) {
            // L'ordre reflète la dernière consultation ; aucune requête active n'est évincée.
            const oldest = [...entries.values()].find(entry => entry.status !== 'loading');
            if (oldest) clearEntry(oldest);
        }
        entries.set(url, {url, status: 'queued'}); pump();
    };
    const cancelIntent = () => {if (intent !== undefined) clearTimeout(intent); intent = undefined;};
    return {
        warm(value: string | null) {
            cancelIntent();
            if (disposed) return;
            const url = collectionImageUrl(value);
            if (url) intent = setTimeout(() => {intent = undefined; enqueue(url);}, INTENT_MS);
        },
        cancelIntent,
        dispose() {
            if (disposed) return;
            disposed = true; cancelIntent();
            for (const entry of entries.values()) clearEntry(entry);
            entries.clear();
        },
    };
}

/** À monter sur CollectionScreen : le démontage et le changement de compte vident la file. */
export function useSkinImagePreloader(accountKey: string | null) {
    const session = useRef<{key: string | null; value: ReturnType<typeof createSkinImagePreloader>} | null>(null);
    useEffect(() => {
        const value = createSkinImagePreloader(); session.current = {key: accountKey, value};
        return () => {value.dispose(); if (session.current?.value === value) session.current = null;};
    }, [accountKey]);
    const warm = useCallback((url: string | null) => {if (session.current?.key === accountKey) session.current.value.warm(url);}, [accountKey]);
    const cancelIntent = useCallback(() => {if (session.current?.key === accountKey) session.current.value.cancelIntent();}, [accountKey]);
    return {warm, cancelIntent};
}
