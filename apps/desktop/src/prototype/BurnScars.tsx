import {useId} from 'react';
/** Matière locale : reflet passager sous le texte, puis deux bords altérés et immobiles. */
export function BurnScars() {
  const glowId=useId();
  const lower="M244 239 C259 233 267 241 282 237 Q291 230 300 233 Q310 239 326 234 C342 224 350 235 364 231 Q371 222 382 228 Q396 235 412 226 C426 219 433 233 445 228 Q461 219 472 230 C485 238 492 225 504 231 Q521 237 535 230 C551 226 556 237 570 233 Q587 228 600 234";
  const upper="M414 1 C434 10 437 1 451 5 Q461 13 475 8 C491 3 499 18 514 12 Q528 8 538 17 Q549 24 560 21 C576 19 574 34 584 37 Q594 41 590 54 Q590 61 600 72";
  return <div className="session-material" aria-hidden="true">
    <div className="surface-sweep" />
    <div className="material-warmth" />
    <svg className="material-scars" viewBox="0 0 600 240" preserveAspectRatio="none">
      <defs><filter id={glowId} x="-30%" y="-100%" width="160%" height="300%" colorInterpolationFilters="sRGB"><feGaussianBlur stdDeviation="4"/></filter></defs>
      <path className="material-ash" d={`${lower} L600 240 Z`} />
      <path className="material-ash" d={`${upper} L600 0 Z`} />
      {[lower,upper].map((path,i)=><g key={i}>
        <path className="material-bloom" d={path} filter={`url(#${glowId})`}/>
        <path className="material-rim" d={path}/>
        <path className="material-ember" d={path}/>
      </g>)}
      <path className="material-fracture" d="M397 234 l7 -6 -3 -5 9 -5 M489 234 l-5 -7 4 -4 M533 10 l7 9 -4 5 8 7"/>
    </svg>
  </div>;
}
