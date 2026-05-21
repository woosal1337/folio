# Accessibility Standards

Attune is a desktop app, but every screen is HTML. The rules from the web
apply: keyboard reachable, screen-reader friendly, motion-considerate.

## Keyboard

- Every interactive element is keyboard-reachable. Buttons, links,
  inputs, switches all participate in the tab order by default.
- Visible focus is mandatory. The `focus-visible:ring-*` utilities in
  shadcn primitives provide it; do not strip them off.
- Custom interactive elements (a `<div>` acting as a button) are
  forbidden. Use `<button>` or `<a>`; if you must build something
  custom, give it `role`, `tabIndex={0}`, and keyboard handlers for
  Enter and Space.
- Modal dialogs trap focus and restore it on close. Radix's `Dialog`
  primitive handles this; do not override the focus logic.
- ESC closes the active dialog. Mouse-click outside closes it too.
  Both are Radix defaults.

## Screen readers

- Icon-only buttons require `aria-label`. The toggle-theme button is
  the canonical example.
- Heading levels are coherent per screen: `<h1>` for the route title,
  `<h2>` for sub-sections. Don't skip levels.
- Live regions: errors that appear after an action use `role="status"`
  for non-critical updates ("Saving…") and `role="alert"` for failures.
- Decorative images use `alt=""`. Meaningful images use a real `alt`.
  Icons from `lucide-react` have no inherent label — pair them with
  text or `aria-label`.
- Status indicators (the recording pulse) include a textual label, not
  just colour. The pulse dot is accompanied by `recording` / `idle`
  text.

## Forms

- Every input has a `<label htmlFor=…>` or wraps it. shadcn's `Label`
  component is the standard.
- Group related inputs in a `<fieldset>` when grouping is semantic
  (e.g. radio choices). The "Provider" selector in the settings modal
  is a candidate for a fieldset upgrade.
- Required fields use `required` on the input, not a custom asterisk.
- Validation errors are read by screen readers via `aria-invalid` and
  `aria-describedby` pointing to the message element.

## Motion

- `prefers-reduced-motion: reduce` is respected. When animations are
  meaningful (the recording pulse), provide a non-animated fallback
  state.
- Tailwind utilities are easy: `motion-reduce:transition-none`,
  `motion-reduce:animate-none` on animated elements.
- Don't introduce auto-playing animations that cover the whole screen.
  The current keyframes (`pulse-record`, accordion-up/down) are local
  and short; keep new ones at that scope.

## Colour and contrast

- Light theme: foreground / background contrast is WCAG AA at minimum
  (4.5:1 for body text, 3:1 for UI). The sage/cream palette is verified.
- Dark theme: same standard. The token values in `globals.css` are
  pre-checked.
- Never communicate state with colour alone. Error states pair red with
  an icon and text. Recording state pairs the destructive colour with
  a label.

## Drag regions

- `[data-drag]` regions cover non-interactive surfaces. Interactive
  children explicitly opt out via `data-no-drag` or are excluded by
  the default selector list in `use-window-drag.ts`.
- Drag does not consume keyboard focus. Macros that simulate a drag
  via the keyboard are not in scope.

## Text scaling

- Use rem-based sizing (Tailwind's defaults). The `2xs` size (0.6875rem)
  is the smallest we ship. Smaller would fail platform defaults at
  200% zoom.
- Truncated text (`truncate`) requires a `title` attribute to expose
  the full value on hover, or a popover that reveals it.
- Long file paths in the Settings modal break with `break-all`. Long
  meeting labels in the recording list use `truncate` with the full
  label in the file name.

## Focus management

- Programmatic focus moves are explicit: clicking "Save" in the
  Settings modal returns focus to the trigger button when the modal
  closes. Radix handles this for `Dialog`.
- Custom navigation between sections (the Settings rail) should move
  focus into the new section's first focusable element. The current
  implementation could improve here; flagged for follow-up.

## Testing

- Manual sweep before each release:
  1. Tab through every screen. Every action reachable.
  2. Activate every button with Enter and Space.
  3. Run VoiceOver (`Cmd+F5`). Headings, landmarks, form labels announce.
  4. Set "Reduce motion" in Accessibility settings. Verify no animation
     loops persist.
- Automated checks: `axe-core` against the rendered Vite dev server.
  Tracked in `tooling.md` as a release-gate.

## Anti-patterns we do not ship

- `tabIndex={-1}` on focusable elements to hide them. Hide the element.
- `outline: none` without a replacement focus ring.
- "Click here" link text. Links describe their destination.
- Tooltips as the only carrier of an action's name.
- `placeholder` as a label substitute. Placeholders disappear; labels
  do not.
