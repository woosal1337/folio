# TypeScript Standards

TypeScript 5.7 with `strict: true`. The compiler is on our side; we lean
into that.

## Compiler flags

`tsconfig.json` is canonical. The flags that matter:

| Flag | Setting | Why |
| --- | --- | --- |
| `strict` | `true` | Implies `noImplicitAny`, `strictNullChecks`, `strictBindCallApply`, etc. |
| `noUnusedLocals` | `true` | Dead code is a bug surface. |
| `noUnusedParameters` | `true` | Prefix with `_` to opt out. |
| `noFallthroughCasesInSwitch` | `true` | Fall-through is almost always a bug. |
| `noUncheckedSideEffectImports` | `true` | Catches accidental `import "foo"`. |
| `useDefineForClassFields` | `true` | Match ECMAScript class field semantics. |
| `moduleResolution` | `bundler` | Required for Vite. |
| `allowImportingTsExtensions` | `true` | Lets us write `import "./foo.ts"` when bundling. |
| `isolatedModules` | `true` | Required for Vite / esbuild. |
| `moduleDetection` | `force` | Treats every file as a module. |
| `noEmit` | `true` | Vite handles emit. |

Do not enable `noPropertyAccessFromIndexSignature` or `noUncheckedIndexedAccess`
yet; the cost-benefit isn't there for v0. Revisit once the codebase grows.

## Types vs interfaces

- `interface` for object shapes that may be extended or merged. Most of
  our model types use `interface`.
- `type` for unions, intersections, mapped/conditional types, and
  function types.
- Both compile to the same runtime; choose based on the next change
  someone will make.

## Type-only imports

- `import type { Foo } from "./bar"` when only the type is needed. The
  bundler drops these at build time even with `isolatedModules`.
- Mixed imports use `import { x, type Y } from "./z"`. The inline `type`
  modifier keeps the runtime import minimal.

## Nullability

- Prefer `T | null` over `T | undefined` for fields that may not be set.
  We mirror Rust `Option<T>`, which serialises to `null`.
- Function returns use `T | null` for "lookup failed" and `T | undefined`
  for "this argument was optional and not provided". The distinction is
  small but consistent.
- Default function parameters use `?:` syntax with optional rest spread
  rather than `| undefined`.

## any and unknown

- `any` is forbidden in new code. Use `unknown` and narrow.
- Library escape hatches occasionally require `as any`; isolate them
  behind a typed wrapper.
- `unknown` at the boundary, refined inside (a Tauri command result is
  typed `Promise<T>` because we set the generic; that's not `any`).

## Exhaustiveness

- Use `never` to enforce exhaustiveness in switch statements over union
  types:

  ```ts
  function describe(c: Channel): string {
    switch (c) {
      case "mic": return "Microphone";
      case "system": return "System";
      default: {
        const _exhaustive: never = c;
        return _exhaustive;
      }
    }
  }
  ```

  Adding a new `Channel` variant fails the type check until every switch
  handles it.

## Discriminated unions

- Status objects use a `kind` (or `status`) tag:

  ```ts
  type LoadState<T> =
    | { kind: "idle" }
    | { kind: "loading" }
    | { kind: "ready"; value: T }
    | { kind: "error"; message: string };
  ```

  Pattern-matchable in switch, type-safe in JSX (`state.kind === "ready"`
  narrows `state.value`).

## Async

- `async/await` everywhere. Promise chains are a code smell.
- Top-level `await` is only in modules where the bundler supports it
  (currently `scripts/rasterize-icon.mjs`). React effects should not
  return promises directly.
- `Promise.all` for independent parallel calls. Don't await sequentially
  for things that can run in parallel.
- Catch errors at the boundary that has the user context. Library wrapper
  functions throw; React components catch and display.

## Errors

- Throw `Error` (or a subclass) with a useful message. Don't throw
  strings.
- The `String(e)` pattern in catch blocks is acceptable as a
  user-display fallback, but log the raw error via `console.error` so
  the structured form survives in the WebKit console.

## Generics

- Type parameters use single uppercase letters (`T`, `K`, `V`) for
  generic-like-Array uses, descriptive names (`TSettings`, `TError`) for
  domain types.
- Constrain generics whenever possible. `<T extends Settings>` beats
  `<T>` because callers see what's accepted.

## Module structure

- Each module exports its public surface and nothing else. No "barrel"
  files (`index.ts` that re-exports everything) — they break tree
  shaking and obscure dependencies.
- Imports ordered: builtin / external first, then `@/` alias, then
  relative. Within each group, ordered alphabetically. Prettier handles
  this via the `prettier-plugin-organize-imports` plugin when wired.

## Naming

- Components: `PascalCase` (`AudioPlayer`, `SettingsModal`).
- Hooks: `camelCase` starting with `use` (`useRecording`).
- Files: `kebab-case` (`audio-player.tsx`, `use-recording.ts`).
- Local variables, functions, properties: `camelCase`.
- Type-level identifiers: `PascalCase`.
- Constants in module scope: `UPPER_SNAKE_CASE` for true constants;
  `camelCase` for typed values that happen to be constant.

## Re-exports

- Re-export only types that consumers genuinely need. Internal types
  stay internal.
- `export *` is allowed when the re-exported module is small and the
  intent is "this is the public surface".

## Comments

- TSDoc (`/** ... */`) on exported functions, types, and complex
  internal helpers. Tools like the LSP and Storybook surface these.
- Inline `//` comments for non-obvious *why*. Don't restate the code.
- Don't include implementation details that will rot ("currently uses
  X; will switch to Y soon"). Track in an issue.

## Forbidden patterns

- `// @ts-ignore` and `// @ts-expect-error` without a comment explaining
  what's being suppressed. `// @ts-expect-error: foo is typed wrong in
  the upstream lib, see issue #N` is fine; bare suppressions are not.
- `Function` as a type — use `(args) => returnType` instead.
- `Object` as a type — use `Record<string, unknown>` or be specific.
- `void` as a value — it's a type, not a value to return.
