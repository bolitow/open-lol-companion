/** Un seul envoi en vol ; les mesures intermédiaires sont remplacées par la dernière.
 * Une mesure n'est acquittée qu'après succès, avec trois tentatives au total. */
export function createContentSizer(send: (height: number) => Promise<void>): {update: (height: number) => void; dispose: () => void} {
    let disposed = false, sending = false, version = 0, failures = 0;
    let latest: number | null = null, acknowledged: number | null = null;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const clearRetry = () => {if (timer !== undefined) clearTimeout(timer); timer = undefined;};
    const pump = () => {
        if (disposed || sending || timer !== undefined || failures >= 3 || latest === null || latest === acknowledged) return;
        const height = latest, sentVersion = version;
        sending = true;
        void (async () => {
            try {
                await send(height);
                if (!disposed) acknowledged = height;
            } catch {
                // Un échec de l'ancienne mesure ne retarde pas sa remplaçante.
                if (!disposed && version === sentVersion) {
                    failures++;
                    if (failures < 3) timer = setTimeout(() => {timer = undefined; pump();}, 250);
                }
            } finally {
                sending = false;
                pump();
            }
        })();
    };
    return {
        update(height) {
            if (disposed || !Number.isFinite(height) || height < 48 || height > 1200) return;
            if (height !== latest) {
                latest = height; version++; failures = 0; clearRetry();
            }
            pump();
        },
        dispose() {disposed = true; latest = null; clearRetry();},
    };
}
