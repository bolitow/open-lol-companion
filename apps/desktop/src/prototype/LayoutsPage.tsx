import layouts from "../../public/prototype/layouts/manifest.json";
import {draftCopy,figmaLayout} from "./draftCopy";
import type {Locale} from "./model";
export function LayoutsPage({locale,onBack}:{locale:Locale;onBack:()=>void}){
 const t=draftCopy[locale];
 return <main className="layouts-page"><div className="draft-heading"><div><span className="caption">Figma · 30.09.2026</span><h1>{t.gallery}</h1></div><button className="outline-button" onClick={onBack}>{t.galleryBack}</button></div><p>{t.galleryHint}</p><a href={figmaLayout} target="_blank" rel="noreferrer">{t.figma}</a><div className="layout-gallery">{layouts.map((layout,i)=><figure key={layout.id}><a href={`/prototype/layouts/${layout.file}`} target="_blank" rel="noreferrer"><img src={`/prototype/layouts/${layout.file}`} alt={`${i+1} · ${layout.title}`} width={1440} height={900} loading="lazy"/></a><figcaption><span>{layout.title}</span><a href={layout.figmaUrl} target="_blank" rel="noreferrer">Figma</a></figcaption></figure>)}</div></main>;
}
