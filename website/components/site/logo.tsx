import { cn } from "@/lib/utils";
import { siteConfig } from "@/lib/site-config";

export function FolioMark({ className }: { className?: string }) {
    return (
        <svg
            viewBox="0 0 1024 1024"
            fill="none"
            role="img"
            aria-label={siteConfig.name}
            className={cn("h-7 w-7", className)}
        >
            <defs>
                <radialGradient
                    id="folio-orb-light"
                    cx="0"
                    cy="0"
                    r="1"
                    gradientUnits="userSpaceOnUse"
                    gradientTransform="translate(382 276) rotate(58) scale(474 560)"
                >
                    <stop offset="0" stopColor="#F0F0F0" />
                    <stop offset="0.28" stopColor="#B0B0B8" />
                    <stop offset="0.62" stopColor="#4A4A52" />
                    <stop offset="1" stopColor="#18181B" />
                </radialGradient>
                <linearGradient
                    id="folio-wave-light"
                    x1="280"
                    y1="486"
                    x2="744"
                    y2="568"
                    gradientUnits="userSpaceOnUse"
                >
                    <stop offset="0" stopColor="#F4F4F5" />
                    <stop offset="0.5" stopColor="#EBEBEB" />
                    <stop offset="1" stopColor="#D4D4D8" />
                </linearGradient>
                <linearGradient
                    id="folio-tile"
                    x1="162"
                    y1="130"
                    x2="872"
                    y2="936"
                    gradientUnits="userSpaceOnUse"
                >
                    <stop offset="0" stopColor="#18181B" />
                    <stop offset="0.58" stopColor="#121216" />
                    <stop offset="1" stopColor="#0E0E10" />
                </linearGradient>
                <filter
                    id="folio-orb-shadow"
                    x="174"
                    y="144"
                    width="678"
                    height="722"
                    filterUnits="userSpaceOnUse"
                    colorInterpolationFilters="sRGB"
                >
                    <feDropShadow
                        dx="0"
                        dy="32"
                        stdDeviation="34"
                        floodColor="#000000"
                        floodOpacity="0.46"
                    />
                    <feDropShadow
                        dx="0"
                        dy="10"
                        stdDeviation="16"
                        floodColor="#303034"
                        floodOpacity="0.22"
                    />
                </filter>
                <clipPath id="folio-orb-clip">
                    <circle cx="512" cy="500" r="274" />
                </clipPath>
            </defs>
            <rect
                x="80"
                y="80"
                width="864"
                height="864"
                rx="212"
                fill="url(#folio-tile)"
            />
            <rect
                x="81.5"
                y="81.5"
                width="861"
                height="861"
                rx="210.5"
                stroke="#303034"
                strokeWidth="3"
                opacity="0.8"
            />
            <g filter="url(#folio-orb-shadow)">
                <circle cx="512" cy="500" r="274" fill="url(#folio-orb-light)" />
                <g clipPath="url(#folio-orb-clip)">
                    <path
                        d="M228 506C290 464 344 462 400 505C462 552 521 552 582 505C642 459 696 462 774 506"
                        stroke="url(#folio-wave-light)"
                        strokeWidth="52"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                    />
                    <path
                        d="M273 594C346 710 493 769 638 710"
                        stroke="#0E0E10"
                        strokeWidth="44"
                        strokeLinecap="round"
                        opacity="0.16"
                    />
                </g>
            </g>
        </svg>
    );
}

export function Logo({
    className,
    showWordmark = true,
}: {
    className?: string;
    showWordmark?: boolean;
}) {
    return (
        <span className={cn("inline-flex items-center gap-2.5", className)}>
            <FolioMark className="h-7 w-7 shrink-0" />
            {showWordmark && (
                <span className="font-wordmark text-2xl font-medium leading-none tracking-tight">
                    {siteConfig.wordmark}
                </span>
            )}
        </span>
    );
}
