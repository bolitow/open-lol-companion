import {useLayoutEffect, useRef, useState} from 'react';
import {Icon} from '../../ui/Icon';
import {collectionImageUrl} from './collectionModel';
import './appcollectionmedia.css';

export interface SkinImageProps {
    url: string | null;
    /** Libellé de repli déjà traduit par l'appelant. */
    label: string;
    /** Miniature existante de la grille, conservée pendant l'arrivée du splash. */
    placeholderUrl?: string | null;
    /** Fiche ouverte et huit premières cartes seulement. */
    priority?: boolean;
}
type ImageState = 'loading' | 'ready' | 'failed';
function SkinImageContent({url, label, placeholderUrl, priority = false}: SkinImageProps) {
    const image = useRef<HTMLImageElement>(null), placeholder = useRef<HTMLImageElement>(null);
    const [status, setStatus] = useState<ImageState>(url ? 'loading' : 'failed');
    const [previewStatus, setPreviewStatus] = useState<ImageState>(placeholderUrl ? 'loading' : 'failed');
    // Une ressource déjà en cache peut être complète avant la pose du gestionnaire load.
    useLayoutEffect(() => {
        if (image.current?.complete && image.current.naturalWidth > 0) setStatus('ready');
        if (placeholder.current?.complete && placeholder.current.naturalWidth > 0) setPreviewStatus('ready');
    }, []);
    const loading = status === 'loading' || (status === 'failed' && previewStatus === 'loading');
    const available = status === 'ready' || previewStatus === 'ready';
    return <span className={`collection-skin-media ${status === 'ready' ? 'is-ready' : ''}`} aria-busy={loading}>
        {loading && previewStatus !== 'ready' && <span className="collection-skin-skeleton" aria-hidden="true"/>}
        {placeholderUrl && previewStatus !== 'failed' && <img ref={placeholder} className={`collection-skin-placeholder ${previewStatus === 'ready' ? 'is-ready' : ''}`} src={placeholderUrl} alt="" loading={priority ? 'eager' : 'lazy'} decoding="async" fetchPriority="auto" referrerPolicy="no-referrer" onLoad={() => setPreviewStatus('ready')} onError={() => setPreviewStatus('failed')}/>}
        {url && status !== 'failed' && <img ref={image} className="collection-skin-image" src={url} alt="" loading={priority ? 'eager' : 'lazy'} decoding="async" fetchPriority={priority ? 'high' : 'auto'} referrerPolicy="no-referrer" onLoad={() => setStatus('ready')} onError={() => setStatus('failed')}/>}
        {!loading && !available && <span className="collection-image-fallback" role="img" aria-label={label}><Icon name="sparkles" size={30}/></span>}
    </span>;
}
/** Le parent garde son ratio fixe ; aucune dimension de la grille ne dépend de l'image. */
export function SkinImage({url, placeholderUrl, ...props}: SkinImageProps) {
    const safe = collectionImageUrl(url), preview = collectionImageUrl(placeholderUrl ?? null);
    const source = safe ?? preview, placeholder = preview && preview !== source ? preview : null;
    return <SkinImageContent key={`${source ?? ''}:${placeholder ?? ''}`} {...props} url={source} placeholderUrl={placeholder}/>;
}
