export const siteConfig = {
    name: "Folio",
    wordmark: "Folio",
    tagline: "Local-first meeting notes for macOS",
    description:
        "Folio captures system audio and your microphone, transcribes on-device, and writes a markdown note per meeting to your own vault. Audio never leaves your machine.",
    url: "https://folio.chele.bi",
    version: "1.1.0",
    license: "Apache-2.0",
    platform: "macOS 13+",
    links: {
        github: "https://github.com/woosal1337/folio",
        releases: "https://github.com/woosal1337/folio/releases/latest",
        issues: "https://github.com/woosal1337/folio/issues",
        privacy: "https://github.com/woosal1337/folio/blob/main/docs/PRIVACY.md",
        architecture:
            "https://github.com/woosal1337/folio/blob/main/docs/ARCHITECTURE.md",
        license: "https://github.com/woosal1337/folio/blob/main/LICENSE",
    },
    install: {
        tapCommand: "brew tap woosal1337/folio https://github.com/woosal1337/folio",
        installCommand: "brew install --cask folio",
        upgradeCommand: "brew upgrade --cask folio",
    },
} as const;

export type NavItem = {
    label: string;
    href: string;
    external?: boolean;
};

export const mainNav: NavItem[] = [
    { label: "Features", href: "/features" },
    { label: "Docs", href: "/docs" },
    { label: "Privacy", href: "/docs/privacy" },
    { label: "GitHub", href: siteConfig.links.github, external: true },
];

export const footerNav: { title: string; items: NavItem[] }[] = [
    {
        title: "Product",
        items: [
            { label: "Overview", href: "/" },
            { label: "Features", href: "/features" },
            { label: "Install", href: "/docs/installation" },
            {
                label: "Changelog",
                href: `${siteConfig.links.github}/blob/main/CHANGELOG.md`,
                external: true,
            },
        ],
    },
    {
        title: "Documentation",
        items: [
            { label: "Getting started", href: "/docs" },
            { label: "How to use", href: "/docs/how-to-use" },
            { label: "Architecture", href: "/docs/architecture" },
            { label: "Connectors (MCP)", href: "/docs/connectors" },
        ],
    },
    {
        title: "Trust",
        items: [
            { label: "Privacy", href: "/docs/privacy" },
            {
                label: "Security",
                href: `${siteConfig.links.github}/blob/main/SECURITY.md`,
                external: true,
            },
            { label: "License", href: siteConfig.links.license, external: true },
            { label: "FAQ", href: "/docs/faq" },
        ],
    },
];
