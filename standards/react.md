# React Standards

React 18.3, function components only, hooks only. The UI is a thin layer
over Tauri commands; we keep that layer cheap, accessible, and obvious.

## Component shape

- Function components, default-exported from route files (`record.tsx`,
  `library.tsx`, etc.), named-exported from shared components.
- Props typed via a local `interface` or `type`. No prop-drilling beyond
  three levels; lift to a context or a `zustand` store at that point.
- One component per file. Helpers that are only used by one component
  live in the same file under the export. Helpers used in multiple
  places move to `src/components/` or `src/lib/`.

## Hooks

- Custom hooks live in `src/hooks/`, named `use-<kebab>.ts`, exported as
  `useThing()`. Each hook is responsible for one concern.
- Hooks return objects, not tuples, when there are more than two values.
  Object returns survive refactoring; tuple returns break call sites
  when the order changes.
- `useCallback` / `useMemo` are not free. Use them when (a) the result is
  passed to a memoised child or to a hook dep array, or (b) the
  computation is meaningfully expensive. Don't reflexively wrap every
  function in `useCallback`.
- Effects clean up. Either return a cleanup function or use an
  `AbortController` / `cancelled` flag for async work. The "cancelled
  guard" pattern lives in `useRecording` for first-mount sync.

## State

- Local state via `useState` for component-scoped values.
- Cross-component UI state (modal open, current route highlight) uses
  React Router for routing, local component state otherwise.
- Server / Tauri state is fetched in effects. `zustand` is available in
  the dep tree but unused in v0; reach for it when shared state grows
  beyond two consumers.
- No Redux. The state surface is too small to justify the boilerplate.

## File structure

```
src/
  App.tsx               root + router
  main.tsx              entry, ReactDOM.createRoot
  assets/               static SVG / images
  components/
    ui/                 shadcn primitives — DO NOT edit ad hoc; use `pnpm dlx shadcn add`
    audio-player.tsx    feature components
    sidebar.tsx
  hooks/                custom hooks, one per file
  lib/
    api.ts              typed Tauri invoke wrappers
    types.ts            mirror of Rust models
    utils.ts            small pure helpers (cn, formatters)
  routes/               top-level screens, lazy-loaded if they grow
  styles/
    globals.css         tokens + Tailwind base
```

## TypeScript and React

- `React.FC` is banned. Use plain function components with prop types.
  `React.FC` infers `children?: ReactNode` even when you don't want it.
- Event handlers prefer `(e: React.MouseEvent<HTMLDivElement>)` over `any`.
- `as` casts are a code smell. If you find yourself writing `as Foo`,
  prove the type with a runtime check or refactor the source of the
  loose type.
- Imports use the `@/` alias for `src/` (configured in `tsconfig.json`
  + `vite.config.ts`). Relative imports are for siblings inside a folder
  only.

## Lists and keys

- `key` is a stable identifier from the data, never an array index, never
  `Math.random()`. The session_dir path or a UUID is the right thing.
- When mapping over a static list (sidebar items, settings sections),
  the literal `id` is the key.

## Conditional rendering

- `condition && <Foo />` is fine when `condition` is boolean.
- `value ? <A /> : null` is preferred over `value && <A />` when `value`
  can be `0` or `""`; falsy non-booleans render as text.
- Three-way splits use `if`/`else if`/`else` in a render helper or a
  switch, not nested ternaries.

## Refs and DOM

- `useRef` for DOM nodes and mutable values that should not trigger
  re-render. The current-value object stays stable across renders.
- Imperative DOM work (focus, scroll, audio play) goes in effects, not
  during render. Render is pure.

## Async UI

- Buttons that trigger network/IPC must:
  1. Disable while pending (`disabled={busy}`).
  2. Show a loading label when meaningful (`busy ? "Saving…" : "Save"`).
  3. Surface errors in-place, not via `alert()` for anything beyond
     destructive confirms.
- Errors render in a status region that the screen reader announces
  (`role="status"` for non-critical, `role="alert"` for critical).

## Re-renders

- Stable prop references via `useCallback`/`useMemo` only when the
  consumer is memoised. Wrapping passes-through into memo gives nothing.
- `React.memo` is rare. Use it when (a) the component renders frequently
  and (b) profiling shows it as a hotspot.
- Keys that change identity cause subtree unmount. Don't compose keys
  from arbitrary state (e.g. `key={open ? 'a' : 'b'}`) unless that
  unmount is intentional.

## Routing

- `react-router-dom` with `HashRouter`. Hash routing avoids server-rewrite
  configuration and works inside Tauri's `tauri://` origin.
- Route paths are `/record`, `/library`, `/editor`, `/tasks`. The default
  redirect is `/record`.
- Future modal-as-route patterns use search params, not nested routes.

## CSS

- Tailwind classes inline on JSX. No CSS modules; design tokens live in
  `globals.css` and Tailwind reads them via `hsl(var(--name))`.
- `cn(...)` (from `clsx` + `tailwind-merge`) is the only class-composition
  helper. It deduplicates conflicting Tailwind classes; never hand-write
  conflict resolution.
- shadcn primitives live in `src/components/ui/`. They are generated
  starting points; we modify them when the design needs it and check in
  the changes. `pnpm dlx shadcn add` won't overwrite without a prompt.

## Accessibility

See `standards/accessibility.md` for full guidance. Quick rules:

- Every button is a `<button>` element. Clickable `<div>`s lose
  keyboard support and screen-reader semantics.
- Icon-only buttons need `aria-label`.
- Dialogs use Radix primitives, which manage focus and ESC for us.
- Drag handles do not absorb keyboard focus.
- Don't use `tabIndex={-1}` to hide focus; hide the element.

## Performance

- Lazy-load routes when the bundle grows past ~250 KB gzipped. Today the
  tree shakes cleanly under that; we don't pay the complexity yet.
- Images and SVG: use `<img loading="lazy">` for off-screen images.
- The webview is Tauri's bundled WebKit on macOS — generally fast, but
  avoid layout thrash (read in one frame, write in the next).

## Testing

- v0 ships without a frontend test suite. When tests land, use Vitest
  + Testing Library, mock Tauri commands with a thin fake of the API
  layer. Do not mock React or the router.
- Snapshot tests are forbidden. They give false confidence and rot.
