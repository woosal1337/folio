/**
 * Infer a default workspace name from the user's email address.
 *
 * Strips the user portion + the public TLDs and capitalises:
 *   ege@clinora.ai     → "Clinora"
 *   ege@acme.co.uk     → "Acme"
 *   ege@acme.io        → "Acme"
 *   ege@deep-mind.com  → "Deep Mind"  (hyphen → space, each part title-case)
 *
 * Returns an empty string when the email is malformed. Callers
 * should treat that as "ask the user".
 *
 * The matching list of public-suffix TLDs is intentionally short — it
 * does not pretend to be a full PSL implementation. We only need the
 * common cases that show up in the founder + enterprise signup flow.
 * If the input is `foo.bar.example.org` the function returns "Foo"
 * (first label only); that's fine because the user can always edit
 * the field before clicking Continue.
 */

const TWO_PART_PUBLIC_SUFFIXES: ReadonlySet<string> = new Set([
  "co.uk",
  "co.jp",
  "com.au",
  "com.br",
  "com.tr",
  "co.in",
  "co.kr",
  "co.za",
  "com.mx",
  "com.sg",
  "ac.uk",
  "gov.uk",
  "net.au",
  "org.uk",
]);

/**
 * Returns the inferred workspace label for a given email. Empty
 * string when the email cannot be parsed.
 */
export function inferWorkspaceNameFromEmail(email: string): string {
  if (!email || !email.includes("@")) return "";
  const at = email.indexOf("@");
  const domain = email
    .slice(at + 1)
    .toLowerCase()
    .trim();
  if (!domain || domain.includes(" ")) return "";

  const labels = domain.split(".");
  if (labels.length < 2) return "";

  // Drop the public-suffix portion (two-part suffixes first, then
  // single-part TLDs).
  let candidate: string | undefined;
  if (labels.length >= 3) {
    const tail2 = `${labels[labels.length - 2]}.${labels[labels.length - 1]}`;
    if (TWO_PART_PUBLIC_SUFFIXES.has(tail2)) {
      candidate = labels[labels.length - 3];
    }
  }
  if (candidate === undefined) {
    // Default: take the label immediately before the final TLD.
    candidate = labels[labels.length - 2];
  }
  if (!candidate) return "";

  return titleCase(candidate);
}

function titleCase(input: string): string {
  return input
    .split(/[-_]+/)
    .filter((p) => p.length > 0)
    .map((p) => p.charAt(0).toUpperCase() + p.slice(1))
    .join(" ");
}
