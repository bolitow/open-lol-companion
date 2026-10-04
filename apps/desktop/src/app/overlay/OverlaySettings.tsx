import {useEffect, useRef, useState, type FormEvent} from 'react';
import {invoke} from '@tauri-apps/api/core';
import type {OverlayPreferences, OverlayState} from '@olc/shared';
import {overlayCopy} from './copy';
import {defaultOverlayPreferences, validOverlayPreferences} from './preferences';
import {useOverlayState} from './useOverlayState';
import './settings.css';
import {SelectField} from '../../ui/SelectField';

export function OverlaySettings({locale}: {locale: 'fr' | 'en'}) {
    const t = overlayCopy[locale], {state, native, error, receive} = useOverlayState();
    const [draft, setDraft] = useState<OverlayPreferences>({...defaultOverlayPreferences, locale});
    const [dirty, setDirty] = useState(false), [pending, setPending] = useState(false);
    const [result, setResult] = useState<'saved' | 'failed' | null>(null);
    const busy = useRef(false);
    useEffect(() => {if (state && !dirty) setDraft({...state.preferences, locale});}, [state, dirty, locale]);
    const update = (change: Partial<OverlayPreferences>) => {
        setDraft(previous => ({...previous, ...change, locale})); setDirty(true); setResult(null);
    };
    const valid = validOverlayPreferences(draft);
    const save = async (event: FormEvent) => {
        event.preventDefault();
        if (!native || !state || !valid || busy.current) return;
        busy.current = true; setPending(true); setResult(null);
        try {
            const updated = await invoke<OverlayState>('overlay_configure', {preferences: {...draft, locale}});
            receive(updated);
            setDirty(false); setResult(updated.error === 'storage' ? 'failed' : 'saved');
        } catch {setResult('failed');}
        finally {busy.current = false; setPending(false);}
    };
    const preview = async () => {
        if (!native || !state || busy.current || (dirty && !state.preview)) return;
        busy.current = true; setPending(true); setResult(null);
        try {receive(await invoke<OverlayState>('overlay_preview', {enabled: !state.preview}));}
        catch {setResult('failed');}
        finally {busy.current = false; setPending(false);}
    };
    const edit = async (type: 'start' | 'cancel' | 'commit') => {
        if(!native||!state||busy.current)return;
        busy.current=true;setPending(true);setResult(null);
        try{receive(await invoke<OverlayState>('overlay_edit',{action:type==='start'?{type}:{type,session:state.editSession}}));}
        catch{setResult('failed')}finally{busy.current=false;setPending(false)}
    };
    const percent = (value: number) => Number.isFinite(value) ? Math.round(value * 100) : '';
    const status = state?.preferences.exclusiveFullscreen ? t.exclusiveHint : state?.visible ? t.visible : state?.preferences.enabled ? t.hidden : t.disabled;
    return <section className="overlay-settings" aria-labelledby="overlay-settings-title">
        <h2 id="overlay-settings-title">{t.title}</h2><p>{t.description}</p>
        {!native && <p>{t.desktop}</p>}
        {native && !state && !error && <p role="status">{t.loading}</p>}
        <form onSubmit={event => {void save(event);}}>
            <fieldset disabled={!native || !state || pending || state.editSession !== null}>
                <label className="overlay-setting-toggle"><input type="checkbox" checked={draft.enabled} onChange={event => update({enabled: event.target.checked})}/>{t.enable}</label>
                <label className="overlay-setting-toggle"><input type="checkbox" checked={draft.exclusiveFullscreen} onChange={event => update({exclusiveFullscreen: event.target.checked})}/>{t.exclusive}</label>
                <p>{t.exclusiveHint}</p><p>{t.shortcut}</p>
                <div className="overlay-settings-grid">
                    <label>{t.monitor}<input type="number" min={1} max={16} step={1} value={Number.isFinite(draft.monitor) ? draft.monitor + 1 : ''} onChange={event => update({monitor: event.target.valueAsNumber - 1})}/></label>
                    <label>{t.x}<input type="number" min={0} max={90} step={1} value={percent(draft.x)} onChange={event => update({x: event.target.valueAsNumber / 100})}/></label>
                    <label>{t.y}<input type="number" min={0} max={90} step={1} value={percent(draft.y)} onChange={event => update({y: event.target.valueAsNumber / 100})}/></label>
                    <label>{t.width}<input type="number" min={10} max={50} step={1} value={percent(draft.width)} onChange={event => update({width: event.target.valueAsNumber / 100})}/></label>
                    <label>{t.height}<input type="number" min={0} max={80} value={percent(draft.height)} onChange={event=>update({height:event.target.valueAsNumber/100})}/></label>
                    <label>{t.style}<SelectField label={t.style} value={draft.style} options={[{value:"dark",label:t.dark},{value:"solid",label:t.solid}]} onChange={value=>update({style:value as "dark"|"solid"})}/></label>
                    <label>{t.opacity}<input type="number" min={0} max={100} step={1} value={percent(draft.opacity)} onChange={event => update({opacity: event.target.valueAsNumber / 100})}/></label>
                </div>
                <p>{t.positionHint}</p>
                {!valid && <p role="alert">{t.invalid}</p>}
                <div className="overlay-settings-actions">
                    <button type="submit" disabled={!valid}>{pending ? t.saving : t.save}</button>
                    <button type="button" disabled={!state?.preview && (dirty || draft.exclusiveFullscreen)} onClick={() => {void preview();}}>{state?.preview ? t.stopPreview : t.preview}</button>
                </div>
                {dirty && <small>{t.previewHint}</small>}
            </fieldset>
        </form>
        <p>{t.editHint}</p>
        <div className="overlay-settings-actions">
            {state?.editSession != null && <button type="button" disabled={pending} onClick={()=>void edit('commit')}>{t.save}</button>}
            <button type="button" disabled={!native||!state||pending||(dirty&&state.editSession===null)||draft.exclusiveFullscreen} onClick={()=>void edit(state?.editSession!=null?'cancel':'start')}>{state?.editSession!=null?t.cancel:t.edit}</button>
        </div>
        <p role="status">{result ? t[result] : state ? status : ''}</p>
        {(error || state?.error || state?.available === false) && <p role="alert">{state?.error === 'storage' ? t.storage : t.unavailable}</p>}
    </section>;
}
