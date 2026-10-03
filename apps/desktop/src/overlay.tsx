import {StrictMode} from 'react';
import {createRoot} from 'react-dom/client';
import {OverlayWindow} from './app/overlay/OverlayWindow';
import './app/overlay/overlay.css';

createRoot(document.getElementById('root')!).render(<StrictMode><OverlayWindow/></StrictMode>);
