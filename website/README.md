# Folio website

Marketing and documentation site for [Folio](https://folio.chele.bi), the local-first
meeting transcription app for macOS. Live at **[folio.chele.bi](https://folio.chele.bi)**.
Built with Next.js, React, TypeScript, and Tailwind CSS, and styled with Folio's own
design tokens. Lives in `website/` inside the main Folio repository.

## Stack

- **Next.js 15** (App Router) + **React 18**
- **TypeScript**
- **Tailwind CSS 3** with Folio's color, type, and motion tokens
- **Radix UI** primitives (tabs, accordion) and **lucide-react** icons
- **Bun** as the package manager and runtime

## Develop

```sh
bun install
bun run dev
```

The site runs at `http://localhost:3000`.

## Scripts

| Script              | What it does                |
| ------------------- | --------------------------- |
| `bun run dev`       | Start the dev server        |
| `bun run build`     | Production build            |
| `bun run start`     | Serve the production build  |
| `bun run lint`      | Lint with `next lint`       |
| `bun run typecheck` | Type-check without emitting |
| `bun run format`    | Format with Prettier        |

## Structure

```
website/
├── app/                 # routes (landing, features, docs/*)
│   ├── layout.tsx       # fonts, dark theme, header, footer
│   ├── page.tsx         # landing page
│   ├── features/        # features page
│   └── docs/            # documentation (overview, install, usage, architecture, ...)
├── components/
│   ├── ui/              # design-system primitives (button, card, badge, tabs, ...)
│   ├── site/            # header, footer, logo, code blocks, sections
│   ├── landing/         # landing-page sections and visuals
│   └── docs/            # docs shell, sidebar, pager, and content primitives
├── lib/                 # site config, docs navigation, utils
└── public/              # logo and favicon
```

## Design

Dark mode only, with a monochrome palette taken straight from the Folio desktop app
(`src/styles/globals.css` and `tailwind.config.ts` in the repo root): near-black
surfaces, white and gray foreground, no accent color. The logo is Folio's own app
mark, and the wordmark is set in Spectral. Body and headings use the system font
stack (SF Pro on macOS), matching the app.

## Deploy

Deployed on Vercel at **[folio.chele.bi](https://folio.chele.bi)**. A standard Next.js
App Router project: the Vercel project root is set to `website/`, the build command is
the default `next build`, and Bun is detected from `bun.lock`.

## Code style

No source comments, per the Folio convention. Code is kept self-explanatory through
naming and small components. Prose belongs in the docs, not in the source.
