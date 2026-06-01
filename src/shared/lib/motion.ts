/**
 * Motion grammar tokens. v2 finding 018 / GET-52.
 *
 * Mirrors the CSS custom properties defined in `src/styles/globals.css`
 * so React components that need inline `style={{ transition: ... }}`
 * (e.g. transient hover states, drag-drop overlays) reach for the
 * same vocabulary as the static class-based styles.
 *
 * ## Composite-only rule (GET-200, enforced by `npm run lint:motion`)
 *
 * Animate ONLY `transform` (translate/scale/rotate) and `opacity` — they
 * run on the compositor thread, off the main thread. The cost taxonomy:
 *
 *   - **Layout** (height, width, top, left, margin, padding, inset): the
 *     browser re-runs layout for the subtree every frame. Worst. Granola
 *     measured one height transition at 60% CPU / 25% GPU on an M2.
 *   - **Paint** (color, background, box-shadow, fill, border-color): a
 *     repaint every frame. Bad in hot/looping UI; tolerable on a one-off
 *     hover (transition-colors).
 *   - **Composite** (transform, opacity): GPU-only, effectively free — the
 *     only thing to animate in always-on / per-frame / looping surfaces.
 *
 * So a growing bar is `transform: scaleX()`, not animated `width`; a
 * sliding panel is `translateX()`, not `left`. The lint bans the blanket
 * Tailwind transition and `transition-[<layout-prop>]` (+ CSS equivalents).
 * Pair a `DURATIONS.*` constant with an `EASING.*` curve. For lists, FLIP
 * via @dnd-kit's animate-layout-changes is the blessed path — never animate
 * `height: auto`. Justify a rare one-off (a sidebar collapse that truly
 * reflows width) with a `motion-allow` comment.
 *
 * Honour `prefers-reduced-motion`: globals.css clamps every
 * animation-duration / transition-duration to ~0ms when the user
 * has Reduce Motion on, so the constants below are safe to use
 * unconditionally.
 */

export const DURATIONS = {
  fast: 120,
  snap: 200,
  modal: 350,
  deliberate: 480,
} as const;

export const EASING = {
  standard: "cubic-bezier(0.32, 0.72, 0, 1)",
  emphasized: "cubic-bezier(0.2, 0, 0, 1)",
  decelerate: "cubic-bezier(0, 0, 0.2, 1)",
  accelerate: "cubic-bezier(0.3, 0, 1, 1)",
  overshoot: "cubic-bezier(0.34, 1.56, 0.64, 1)",
} as const;

export type MotionDuration = keyof typeof DURATIONS;
export type MotionEasing = keyof typeof EASING;

/**
 * Build a CSS `transition` shorthand. Use this in inline `style={...}`
 * when a Tailwind utility class won't reach: drag overlays, focus
 * outline animations, popover open transitions.
 */
export function transition(
  property: string,
  duration: MotionDuration = "snap",
  easing: MotionEasing = "standard"
): string {
  return `${property} ${DURATIONS[duration]}ms ${EASING[easing]}`;
}

/**
 * True when the OS reports `prefers-reduced-motion: reduce`. Components
 * that orchestrate sequential animations check this to skip the
 * choreography rather than playing it at 0ms per the globals.css clamp.
 */
export function prefersReducedMotion(): boolean {
  if (typeof window === "undefined" || !window.matchMedia) return false;
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}
