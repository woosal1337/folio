export type DocLink = {
    title: string;
    href: string;
    summary: string;
};

export type DocSection = {
    title: string;
    items: DocLink[];
};

export const docsNav: DocSection[] = [
    {
        title: "Getting started",
        items: [
            {
                title: "Overview",
                href: "/docs",
                summary:
                    "What Folio is, the core ideas, and how the pieces fit together.",
            },
            {
                title: "Installation",
                href: "/docs/installation",
                summary:
                    "Install with Homebrew or the notarized DMG, then grant permissions.",
            },
            {
                title: "How to use",
                href: "/docs/how-to-use",
                summary: "From first launch to a searchable vault of meeting notes.",
            },
        ],
    },
    {
        title: "Going deeper",
        items: [
            {
                title: "Architecture",
                href: "/docs/architecture",
                summary:
                    "The Rust core, the capture pipeline, and the data flow, explained.",
            },
            {
                title: "Connectors (MCP)",
                href: "/docs/connectors",
                summary:
                    "Expose transcripts, tasks, and memory to MCP-aware tools, locally.",
            },
            {
                title: "Privacy & consent",
                href: "/docs/privacy",
                summary:
                    "Privacy Mode, the network surface, retention, and recording consent.",
            },
        ],
    },
    {
        title: "Reference",
        items: [
            {
                title: "FAQ",
                href: "/docs/faq",
                summary: "Short answers to the questions people ask before installing.",
            },
        ],
    },
];

export const docsFlat: DocLink[] = docsNav.flatMap((section) => section.items);

export function docPager(pathname: string) {
    const index = docsFlat.findIndex((item) => item.href === pathname);
    if (index === -1) {
        return { prev: null, next: null };
    }
    return {
        prev: index > 0 ? docsFlat[index - 1] : null,
        next: index < docsFlat.length - 1 ? docsFlat[index + 1] : null,
    };
}
