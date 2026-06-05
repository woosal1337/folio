# Frontend architecture — Folio guidelines

Source-cited guidance for the React + TypeScript frontend. Stack: React 18+, Vite, Bun, Zustand, shadcn-style primitives, `ts-rs` for cross-language types. Targets the `src/` tree.

## TL;DR — the rules

1. **Imports flow downward only.** `features/*` can import from `shared/*`; never vice-versa, never between sibling features. ([FSD](https://feature-sliced.design/docs/get-started/overview))
2. **One Zustand value per `useStore` call**, or use `useShallow` for object selectors. ([Sanjewa](https://sanjewa.com/blogs/advanced-zustand-patterns-slices-middleware/))
3. **Only `src/shared/lib/ipc.ts` imports from `@tauri-apps/api/core`.** Every command goes through a typed wrapper.
4. **Cleanup every Tauri `listen` in `useEffect`.** Vite HMR re-runs effects; orphan listeners produce double-renders. ([TauRPC](https://github.com/MatsDK/TauRPC))
5. **Files: kebab-case. Exports: PascalCase components, camelCase hooks/fns, UPPER_SNAKE consts.** No barrel `index.ts` re-exports inside `features/` or `shared/ui/`.
6. **Test stores, test IPC wrappers, test render-and-click smoke flows. Skip third-party primitives, pixel layouts, and `useStore` call counts.**
7. **All color comes from CSS variables, not hard-coded Tailwind shades.** Dark mode = `:root` + `.dark` overrides.

## Architectural layout — pragmatic Feature-Sliced Design

The current `src/` layout (`features/`, `shared/`, plus `chrome/` + entries) is a relaxed [Feature-Sliced Design (FSD)](https://feature-sliced.design/docs/get-started/overview). Knowing the canonical layers helps reason about where new code belongs even when not all are enforced.

| Layer                      | Purpose                                                      | Folio today                                               |
| -------------------------- | ------------------------------------------------------------ | --------------------------------------------------------- |
| `app/`                     | Entrypoints, providers, routing, global styles               | Implicit at `src/main.tsx`, `src/App.tsx`                 |
| `processes/`               | Cross-page flows (deprecated by FSD itself)                  | Not used. Don't adopt.                                    |
| `pages/`                   | Full screens / routes                                        | Implicit — pages currently live inside features           |
| `widgets/`                 | Large self-contained UI blocks composing several features    | Not used. Add only if a block is reused across pages.     |
| `features/`                | User-facing capabilities with business value                 | `src/features/{editor,library,recording,settings,tasks}/` |
| `entities/`                | Domain models (`session`, `transcript`) and their store/UI   | Partial — domain types live in `shared/types/`            |
| `shared/`                  | Reusable, business-agnostic code (UI primitives, IPC, utils) | `src/shared/{lib,stores,types,ui,hooks}/`                 |
| `chrome/` (Folio-specific) | Persistent app chrome (sidebar, drag strip)                  | `src/chrome/`                                             |

### The one rule that matters

From [FSD](https://feature-sliced.design/docs/get-started/overview):

> "Modules on one layer can only know about and import from modules from the layers strictly below."

> "Slices cannot use other slices on the same layer."

Concretely for Folio:

- `features/editor/` must **not** import from `features/settings/`. If they need to share, the shared piece moves down into `shared/` or `entities/`.
- `shared/ui/button.tsx` must **not** import anything from `features/*`.
- Cross-feature composition belongs in `pages/` (today: at the route shell in `App.tsx`) or a future `widgets/` layer.

### How strictly to apply it

From the [Mastering FSD: Lessons from Real Projects](https://dev.to/arjunsanthosh/mastering-feature-sliced-design-lessons-from-real-projects-2ida):

> "For small apps or prototypes, FSD is simply too much... Starting with the basics (App, Pages, Widgets, and Shared) gives you a solid foundation, and then you can gradually add the more complex layers like Features and Entities when your project is ready for them."

**Folio's stance:** keep `app` (implicit) + `features` + `shared` + `chrome`. Add an `entities/` layer the first time two features want to render or mutate the same domain object (today, `Session` and `TranscriptSegment` are the most likely candidates). Add `pages/` if routing grows past three top-level screens. Skip `widgets/` and `processes/` indefinitely.

**Enforcement:** add `eslint-plugin-boundaries` or `eslint-plugin-import` with `no-restricted-paths` to make the import-direction rule a lint error, not a convention. Without ESLint enforcement, the rule rots within a quarter.

## Component design

shadcn/ui is "composition over configuration": you own the source.

### Local vs `shared/ui/`

A primitive earns a place in `src/shared/ui/` when **all three** are true:

1. It has zero business knowledge (no domain types, no IPC).
2. It is used or expected to be used by ≥ 2 features.
3. Its API is stable enough that you would document its props.

Otherwise leave it next to the feature that needs it (`features/editor/components/waveform.tsx`). Premature extraction is worse than duplication — the cost of un-sharing a too-eagerly-shared component is higher than copy-pasting a 40-line component twice.

### Composition patterns

Prefer composition over prop explosions. Two patterns dominate in the shadcn world:

- **Radix `asChild`** — render-as. Pass a child element that the primitive enhances rather than wraps.
- **Render props / slot props** — Base UI's newer approach for custom triggers.

If a component starts collecting `isLoading`, `loadingText`, `loadingIcon`, `loadingPosition` props, that's the signal to split into a composable subtree, not to add a fifth prop.

### Theming and dark mode

From [shadcn/ui Dark Mode](https://ui.shadcn.com/docs/dark-mode):

> "Dark mode works by overriding the same tokens inside a `.dark` selector."

Rules for Folio:

- All color comes from CSS variables (semantic `--background` / `--foreground` pairs). Never hard-code Tailwind colors like `bg-zinc-900`.
- Light/dark live in `:root` and `.dark` inside `src/styles/`.
- Dark mode state is managed by the existing `useTheme` hook + system preference detection.
- For Tauri, forward the OS theme via `getCurrentWindow.theme` on startup so the WebView matches the system at launch.

## State management — Zustand in 2026

### When Zustand vs other tools

For a Tauri desktop app, the "server" is the Rust backend.

- **Zustand** for client state (UI mode, current selection, settings draft, in-progress transcription buffer, modal open/close).
- **TanStack Query** only if you grow list/detail screens that re-fetch from Rust frequently and benefit from caching/pagination/optimistic updates. Today Folio is below that threshold — keep IPC results in Zustand or component state.

### Slice pattern

From [Advanced Zustand Patterns: Slices & Middleware 2026](https://sanjewa.com/blogs/advanced-zustand-patterns-slices-middleware/):

> "The slice pattern combines separate store slices using spread operators and wrapping them with middleware like devtools and persist."

When the store grows beyond ~200 lines, split:

```
src/shared/stores/
  index.ts                 // create() composing slices + middleware
  slices/
    transcription-slice.ts
    settings-slice.ts
    ui-slice.ts
```

Each slice exports `StateCreator<RootState, [], [], SliceState>`. Compose in `index.ts` with `devtools(persist(...))`.

Default to **one global store with slices**. Split into a second store only when a subtree is truly independent (e.g., a modal whose state should not survive close). Multiple stores fragment devtools and complicate cross-state derivations.

### Selectors and re-renders

From [Optimizing React Component Rendering with Zustand](https://medium.com/@nuwan.thuduwage/optimizing-react-component-rendering-with-zustand-stop-re-rendering-what-didnt-change-e538163717e5):

> "Avoid subscribing to entire store and instead use selectors for specific state; when returning objects or arrays, use shallow comparison instead of strict equality checks."

Rules:

- **One value per `useStore` call.** `const status = useStore(s => s.transcription.status)`. Don't return objects.
- For multiple values, use `useShallow` from `zustand/react/shallow`. Zustand v5 throws a runtime warning if you return a fresh object without it.
- Actions live in the store, are stable references, and can be selected by name without `useShallow`.

### Persistence

Use `persist` middleware **only** for things the user expects to survive a relaunch (window position, last opened project, settings). Never persist transient state (transcription buffers, modal open/close). Set an explicit `partialize` to whitelist persisted keys — never persist the whole store.

For Tauri specifically, prefer `@tauri-apps/plugin-store` over `localStorage`. localStorage is volatile in the WebView and gets wiped on certain OS updates.

### Testing stores

Export the slice creator separately so tests can call it with a mock `set`/`get`:

```ts
export const createTranscriptionSlice: StateCreator<
  RootState,
  [],
  [],
  TranscriptionSliceState
> = (set, get) => ({
  /* ... */
});
```

Reset stores between tests:

```ts
beforeEach(() => useStore.setState(initialState, true));
```

## Tauri ↔ React IPC

### Typed command wrappers

Folio's `src/shared/lib/ipc.ts` is the **only** file that should import from `@tauri-apps/api/core`. Every command gets a typed wrapper:

```ts
export async function startTranscription(
  input: StartTranscriptionInput
): Promise<Session> {
  return safeInvoke("start_transcription", input);
}
```

`safeInvoke` does three things: serialize input, call `invoke`, normalize errors (see below).

### `ts-rs` vs `tauri-specta`

Folio currently uses `ts-rs`. It works for one-type-at-a-time exports but has limits:

- No recursive dependent-type export.
- No typed event support.
- Manual TS wrapper functions in `src/shared/lib/ipc.ts`.

`tauri-specta` v2 generates TypeScript for commands AND events, eliminating manual `invoke<T>` and `listen<T>` wrappers. Migrate when:

- IPC surface grows past ~30 commands, or
- You start needing typed events, or
- The manual wrapper duplication becomes painful.

Migration is mechanical: add `#[specta::specta]` next to `#[tauri::command]`, register with `tauri_specta::ts::export`, replace `src/shared/lib/ipc.ts` with the generated `bindings.ts`.

### Error normalization at the boundary

Define a Rust `enum CommandError` with `Serialize` and `thiserror::Error` (see [`tauri-architecture.md`](./tauri-architecture.md) for the full pattern).

In `safeInvoke`, parse the rejection with a Zod schema and re-throw a typed `AppError`:

```ts
const AppErrorSchema = z.object({
  kind: z.enum(["ModelMissing", "SessionBusy", "PermissionDenied", "Other"]),
  message: z.string(),
});

export async function safeInvoke<T>(cmd: string, args?: unknown): Promise<T> {
  try {
    return await invoke<T>(cmd, args as InvokeArgs);
  } catch (e) {
    const parsed = AppErrorSchema.safeParse(e);
    if (parsed.success) throw new AppError(parsed.data);
    throw new AppError({ kind: "Other", message: String(e) });
  }
}
```

React never sees raw Rust strings — it sees `{ kind: 'PermissionDenied', message: '…' }` and renders the matching toast.

### Event listener cleanup (HMR safe)

Always:

```ts
useEffect(() => {
  let unlisten: UnlistenFn | undefined;
  listen("transcription:chunk", handler).then((u) => {
    unlisten = u;
  });
  return () => unlisten?.();
}, []);
```

Wrap in a `useTauriEvent(name, handler)` hook in `src/shared/hooks/use-tauri-event.ts` so contributors cannot forget cleanup. Vite HMR re-runs `useEffect`, and a missing `unlisten` produces duplicate handlers that double-write to Zustand — the classic "every save makes the transcript blink twice" bug.

### Event names

`domain:verb-noun` — `recording:state-changed`, `transcription:partial`, `settings:updated`. Define the strings once in a constants module on both sides; never bare string literals at call sites.

## React 18+ patterns

### Suspense

Use Suspense for code-split routes (`React.lazy`). For data, only adopt Suspense if you bring in TanStack Query with `suspense: true`. Do not write hand-rolled "throw a promise" data hooks — they break devtools and error boundaries.

### `useTransition`

> "By letting urgent updates (typing, clicks, drags) happen instantly while heavier work (filters, charts, big renders) runs 'in the background,' concurrency makes your UI buttery-smooth." — [React 19 Concurrency Deep Dive](https://dev.to/a1guy/react-19-concurrency-deep-dive-mastering-usetransition-and-starttransition-for-smoother-uis-51eo)

Concrete Folio cases:

- Filtering a long transcript list as the user types in search → wrap the search-state setter in `startTransition`.
- Re-rendering the waveform when zoom changes → ditto.

### File naming and barrels

From [Naming Conventions in React](https://www.sufle.io/blog/naming-conventions-in-react):

> "Use kebab-case (e.g., `my-component.tsx`) for file names... prevents naming conflicts on case-insensitive file systems."

> "For React components, the PascalCase convention is recommended."

Folio convention:

- Files: **kebab-case** (`transcript-editor.tsx`, `use-tauri-event.ts`).
- Exports: **PascalCase** components, **camelCase** hooks/functions/values, **UPPER_SNAKE** constants, **PascalCase** types/interfaces.
- **No barrel files (`index.ts` re-exports)** inside `features/` and `shared/ui/`. Barrels defeat Vite tree-shaking in dev, slow HMR, and create circular import traps. Import the exact path.
- Allowed exceptions: the top of `src/shared/types/` (the `ts-rs` output), and the eventual `src/shared/stores/index.ts` that composes slices.

## Testing

### Tooling

- **Vitest** for unit/component, configured with `environment: 'jsdom'` and `globals: true`.
- **@testing-library/react** for rendering, **@testing-library/user-event** for interactions (never fire raw events).
- **vitest-axe** for accessibility assertions on rendered DOM.
- **eslint-plugin-jsx-a11y** at lint time.

### What to test (priority order)

1. **Store logic.** Pure functions — fast, deterministic, highest ROI. Test every reducer/action.
2. **IPC wrappers.** Mock `invoke`, assert error normalization works (`safeInvoke` turns a Rust `CommandError::ModelMissing` into the right discriminated-union case).
3. **Render-and-click smoke tests** for each top-level feature panel — assert it mounts, the primary CTA exists, clicking it dispatches the right store action. Use `screen.getByRole`.
4. **Accessibility smoke** — wrap step 3's rendered tree in `expect(await axe(container)).toHaveNoViolations`.

### What NOT to test

- Internal component state, refs, effect call counts.
- Third-party primitives (Radix, shadcn) — they have their own tests.
- Visual layout / pixel positions — leave to manual review or Playwright screenshot tests later.
- Zustand internals (`useStore` being called) — test the _behavior_ the user sees instead.

### Accessibility, two layers

From [web.dev: Accessibility audit with react-axe and eslint-plugin-jsx-a11y](https://web.dev/articles/accessibility-auditing-react):

> "eslint-plugin-jsx-a11y identifies and enforces accessibility rules directly in your JSX, and when used in combination with a tool that tests the final rendered DOM, such as react-axe, you can find and fix accessibility concerns."

- **Static (lint):** `eslint-plugin-jsx-a11y` with `recommended` config, integrated in existing eslint pipeline.
- **Runtime (test):** `vitest-axe` on the smoke render in step 3.
- **Manual:** keyboard-only navigation pass before releases — automated tools miss focus order and live-region etiquette.

## Quick reference — Do / Don't

| Do                                          | Don't                                                     |
| ------------------------------------------- | --------------------------------------------------------- |
| Import only from layers below               | Cross-import between sibling features                     |
| One value per `useStore` selector           | Return objects without `useShallow`                       |
| Wrap every `invoke` in `safeInvoke`         | Import `@tauri-apps/api/core` outside `shared/lib/ipc.ts` |
| Cleanup every Tauri `listen` in `useEffect` | Leave dangling listeners (HMR will multiply them)         |
| kebab-case filenames, PascalCase exports    | Re-export through `index.ts` barrels                      |
| Test store logic and accessibility          | Test third-party primitives or pixel layouts              |
| Persist whitelisted keys via Tauri Store    | Persist whole store to localStorage                       |
| Use semantic CSS variables for colors       | Hard-code Tailwind color shades                           |

## Folio-specific refactor candidates

- **`src/features/editor/agent-panel.tsx` (418 lines)** — extract sub-components (message list, composer, tool-output rendering) into `features/editor/components/`.
- **`src/features/editor/route.tsx` (360 lines)** — extract toolbar, segment list, and bottom bar into separate components.
- **`src/features/settings/section-ai.tsx` (345 lines)** — extract per-provider sub-panels into `settings/components/`.
- **Wrap `listen` calls** in a `useTauriEvent` hook if not already done. Audit existing `useEffect` blocks for missing `unlisten`.
- **Add `safeInvoke` wrapper** in `src/shared/lib/ipc.ts` with Zod schema for the (future) tagged error type.

## Sources

- [Feature-Sliced Design — Overview](https://feature-sliced.design/docs/get-started/overview)
- [Feature-Sliced Design — Layers reference](https://feature-sliced.design/docs/reference/layers)
- [Mastering FSD: Lessons from Real Projects](https://dev.to/arjunsanthosh/mastering-feature-sliced-design-lessons-from-real-projects-2ida)
- [Denebrix: Feature Sliced Design Guide](https://denebrixai.com/blog/feature-sliced-design-guide/)
- [shadcn/ui — Theming](https://ui.shadcn.com/docs/theming)
- [shadcn/ui — Dark Mode (Vite)](https://ui.shadcn.com/docs/dark-mode/vite)
- [Zustand — GitHub](https://github.com/pmndrs/zustand)
- [Advanced Zustand Patterns: Slices & Middleware 2026](https://sanjewa.com/blogs/advanced-zustand-patterns-slices-middleware/)
- [Zustand DOs and DON'Ts](https://medium.com/@nfailla93/zustand-in-react-dos-and-donts-5a608c26c68)
- [Optimizing React Rendering with Zustand](https://medium.com/@nuwan.thuduwage/optimizing-react-component-rendering-with-zustand-stop-re-rendering-what-didnt-change-e538163717e5)
- [Tauri v2 — Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
- [Adding Type-Safe Commands to Your Tauri Frontend](https://www.gramigna.dev/blog/tauri-type-safety/)
- [tauri-specta — GitHub](https://github.com/specta-rs/tauri-specta)
- [specta — GitHub](https://github.com/specta-rs/specta)
- [TauRPC — GitHub](https://github.com/MatsDK/TauRPC)
- [React 19 Concurrency Deep Dive: useTransition](https://dev.to/a1guy/react-19-concurrency-deep-dive-mastering-usetransition-and-starttransition-for-smoother-uis-51eo)
- [Naming Conventions in React for Clean & Scalable Code](https://www.sufle.io/blog/naming-conventions-in-react)
- [Guide to React Testing Library using Vitest](https://makersden.io/blog/guide-to-react-testing-library-vitest)
- [Bulletproof React Testing with Vitest & RTL](https://vaskort.medium.com/bulletproof-react-testing-with-vitest-rtl-deeaabce9fef)
- [eslint-plugin-jsx-a11y — GitHub](https://github.com/jsx-eslint/eslint-plugin-jsx-a11y)
- [web.dev — Accessibility audit with react-axe and eslint-plugin-jsx-a11y](https://web.dev/articles/accessibility-auditing-react)
