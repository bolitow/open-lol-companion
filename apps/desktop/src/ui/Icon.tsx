import {
    Circle, Clock3, ArrowLeft, ArrowRight, ChartNoAxesCombined, Check, ChevronRight, Clapperboard, Eye,
    Flame as LucideFlame, Info, Menu, Moon, Pin, Play, RotateCcw, Search,
    Settings, Shield, Sun, Swords, UserRound, UsersRound, X, Maximize2, Languages, Sparkles, Zap,
} from 'lucide-react';

// Imports explicites : seules les icônes utilisées entrent dans le bundle.
const icons = {
    menu: Menu, search: Search, arrow: ArrowRight, back: ArrowLeft, chevron: ChevronRight,
    close: X, sun: Sun, moon: Moon, settings: Settings, replay: RotateCcw,
    play: Play, clip: Clapperboard, eye: Eye, sword: Swords,
    circle: Circle, clock: Clock3, chart: ChartNoAxesCombined, check: Check, pin: Pin, info: Info,
    user: UserRound, users: UsersRound, shield: Shield, expand: Maximize2, language: Languages, sparkles: Sparkles, flash: Zap,
} as const;

export function Icon({ name, size = 20 }: { name: keyof typeof icons; size?: number }) {
    const Component = icons[name];
    return <Component size={size} strokeWidth={1.75} aria-hidden="true" focusable="false" />;
}

export function Flame({ size = 30 }: { size?: number }) {
    return <LucideFlame size={size} strokeWidth={1.75} aria-hidden="true" focusable="false" />;
}
