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

export function transition(
  property: string,
  duration: MotionDuration = "snap",
  easing: MotionEasing = "standard"
): string {
  return `${property} ${DURATIONS[duration]}ms ${EASING[easing]}`;
}
