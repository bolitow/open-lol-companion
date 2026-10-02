import {afterEach, expect, it, vi} from 'vitest';
import {createContentSizer} from './contentSize';

const flush = async () => {await Promise.resolve(); await Promise.resolve(); await Promise.resolve();};
afterEach(() => {vi.useRealTimers();});

it('sérialise les commandes et transmet seulement la dernière mesure après la réponse en cours', async () => {
    const heights: number[] = [], replies: (() => void)[] = [];
    const sizer = createContentSizer(height => {heights.push(height); return new Promise(resolve => replies.push(resolve));});
    sizer.update(180); sizer.update(220); sizer.update(240);
    expect(heights).toEqual([180]);
    replies[0]!(); await flush();
    expect(heights).toEqual([180, 240]);
    replies[1]!(); await flush();
    sizer.update(240);
    expect(heights).toEqual([180, 240]);
    sizer.dispose();
});

it('réessaie la même hauteur rejetée après 250 ms et ne l’acquitte qu’après réussite', async () => {
    vi.useFakeTimers();
    const heights: number[] = [];
    const sizer = createContentSizer(async height => {heights.push(height); if (heights.length === 1) throw new Error('temporary');});
    sizer.update(200); await flush();
    expect(heights).toEqual([200]);
    await vi.advanceTimersByTimeAsync(249);
    expect(heights).toEqual([200]);
    await vi.advanceTimersByTimeAsync(1);
    expect(heights).toEqual([200, 200]);
    sizer.update(200); await vi.advanceTimersByTimeAsync(1000);
    expect(heights).toEqual([200, 200]);
    sizer.dispose();
});

it('borne une panne persistante à trois tentatives pour une mesure', async () => {
    vi.useFakeTimers();
    const heights: number[] = [];
    const sizer = createContentSizer(async height => {heights.push(height); throw new Error('offline');});
    sizer.update(180); await flush();
    await vi.advanceTimersByTimeAsync(5000);
    expect(heights).toEqual([180, 180, 180]);
    sizer.update(180); await vi.advanceTimersByTimeAsync(1000);
    expect(heights).toEqual([180, 180, 180]);
    sizer.dispose();
});

it('remplace une nouvelle tentative ancienne par la mesure récente', async () => {
    vi.useFakeTimers();
    const heights: number[] = [];
    const sizer = createContentSizer(async height => {heights.push(height); if (height === 180) throw new Error('temporary');});
    sizer.update(180); await flush();
    sizer.update(250); await flush(); await vi.advanceTimersByTimeAsync(1000);
    expect(heights).toEqual([180, 250]);
    sizer.dispose();
});

it('ignore les mesures invalides et supprime la commande suivante lors du démontage', async () => {
    const heights: number[] = [];
    let reply!: () => void;
    const sizer = createContentSizer(height => {heights.push(height); return new Promise(resolve => {reply = resolve;});});
    for (const height of [NaN, Infinity, -1, 47, 1201]) sizer.update(height);
    expect(heights).toEqual([]);
    sizer.update(48); sizer.update(1200);
    expect(heights).toEqual([48]);
    sizer.dispose(); reply(); await flush(); sizer.update(300);
    expect(heights).toEqual([48]);
});

it('annule les nouvelles tentatives au démontage, même si le rejet arrive ensuite', async () => {
    vi.useFakeTimers();
    const heights: number[] = [];
    let reject!: (reason: Error) => void;
    const sizer = createContentSizer(height => {heights.push(height); return new Promise((_resolve, fail) => {reject = fail;});});
    sizer.update(180);
    expect(heights).toEqual([180]);
    sizer.dispose(); reject(new Error('late')); await flush(); await vi.advanceTimersByTimeAsync(1000);
    expect(heights).toEqual([180]);
});
