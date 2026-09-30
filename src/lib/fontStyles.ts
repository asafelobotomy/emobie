/**
 * Unicode "font" styles for macro expansions.
 *
 * Like online font generators, these are not real fonts: each style swaps
 * plain letters/digits for look-alike Unicode characters, so the result
 * survives copy/paste and typing into any app. Characters a style has no
 * glyph for (punctuation, emoji, accents) pass through unchanged.
 */

export type FontStyleId =
  | "bold"
  | "italic"
  | "boldItalic"
  | "script"
  | "boldScript"
  | "fraktur"
  | "boldFraktur"
  | "doubleStruck"
  | "sans"
  | "sansBold"
  | "sansItalic"
  | "sansBoldItalic"
  | "monospace"
  | "circled"
  | "negativeCircled"
  | "squared"
  | "negativeSquared"
  | "fullwidth"
  | "smallCaps"
  | "superscript"
  | "upsideDown"
  | "strikethrough"
  | "underline";

export type FontStyle = {
  id: FontStyleId;
  label: string;
  transform: (text: string) => string;
};

const UPPER = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWER = "abcdefghijklmnopqrstuvwxyz";
const DIGITS = "0123456789";

type Ranges = {
  upper?: number;
  lower?: number;
  digit?: number;
  /** Letters whose code point is not upper/lower + offset (reserved holes). */
  exceptions?: Record<string, string>;
};

function rangeMapper({ upper, lower, digit, exceptions = {} }: Ranges) {
  const table = new Map<string, string>();
  const fill = (source: string, base: number | undefined) => {
    if (base === undefined) return;
    [...source].forEach((ch, i) => table.set(ch, String.fromCodePoint(base + i)));
  };
  fill(UPPER, upper);
  fill(LOWER, lower);
  fill(DIGITS, digit);
  for (const [from, to] of Object.entries(exceptions)) table.set(from, to);
  return (text: string) =>
    [...text].map((ch) => table.get(ch) ?? ch).join("");
}

function tableMapper(from: string, to: string) {
  const source = [...from];
  const target = [...to];
  const table = new Map(source.map((ch, i) => [ch, target[i]]));
  return (text: string) =>
    [...text].map((ch) => table.get(ch) ?? ch).join("");
}

/** Append a combining mark after every non-whitespace character. */
function combining(mark: string) {
  return (text: string) =>
    [...text].map((ch) => (/\s/.test(ch) ? ch : ch + mark)).join("");
}

const upsideDownMap = tableMapper(
  "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.,!?'()[]<>&_",
  "ɐqɔpǝɟƃɥᴉɾʞlɯuodbɹsʇnʌʍxʎz∀ꓭƆᗡƎℲ⅁HIſꓘ⅂WNOԀꝹꓤS⊥∩ΛMX⅄Z0ƖᄅƐㄣϛ9ㄥ86˙'¡¿,)(][><⅋‾",
);

function upsideDown(text: string): string {
  return text
    .split("\n")
    .map((line) => [...upsideDownMap(line)].reverse().join(""))
    .join("\n");
}

export const FONT_STYLES: FontStyle[] = [
  { id: "bold", label: "Bold", transform: rangeMapper({ upper: 0x1d400, lower: 0x1d41a, digit: 0x1d7ce }) },
  { id: "italic", label: "Italic", transform: rangeMapper({ upper: 0x1d434, lower: 0x1d44e, exceptions: { h: "ℎ" } }) },
  { id: "boldItalic", label: "Bold italic", transform: rangeMapper({ upper: 0x1d468, lower: 0x1d482 }) },
  {
    id: "script",
    label: "Script",
    transform: rangeMapper({
      upper: 0x1d49c,
      lower: 0x1d4b6,
      exceptions: { B: "ℬ", E: "ℰ", F: "ℱ", H: "ℋ", I: "ℐ", L: "ℒ", M: "ℳ", R: "ℛ", e: "ℯ", g: "ℊ", o: "ℴ" },
    }),
  },
  { id: "boldScript", label: "Bold script", transform: rangeMapper({ upper: 0x1d4d0, lower: 0x1d4ea }) },
  {
    id: "fraktur",
    label: "Fraktur",
    transform: rangeMapper({
      upper: 0x1d504,
      lower: 0x1d51e,
      exceptions: { C: "ℭ", H: "ℌ", I: "ℑ", R: "ℜ", Z: "ℨ" },
    }),
  },
  { id: "boldFraktur", label: "Bold fraktur", transform: rangeMapper({ upper: 0x1d56c, lower: 0x1d586 }) },
  {
    id: "doubleStruck",
    label: "Double-struck",
    transform: rangeMapper({
      upper: 0x1d538,
      lower: 0x1d552,
      digit: 0x1d7d8,
      exceptions: { C: "ℂ", H: "ℍ", N: "ℕ", P: "ℙ", Q: "ℚ", R: "ℝ", Z: "ℤ" },
    }),
  },
  { id: "sans", label: "Sans", transform: rangeMapper({ upper: 0x1d5a0, lower: 0x1d5ba, digit: 0x1d7e2 }) },
  { id: "sansBold", label: "Sans bold", transform: rangeMapper({ upper: 0x1d5d4, lower: 0x1d5ee, digit: 0x1d7ec }) },
  { id: "sansItalic", label: "Sans italic", transform: rangeMapper({ upper: 0x1d608, lower: 0x1d622 }) },
  { id: "sansBoldItalic", label: "Sans bold italic", transform: rangeMapper({ upper: 0x1d63c, lower: 0x1d656 }) },
  { id: "monospace", label: "Monospace", transform: rangeMapper({ upper: 0x1d670, lower: 0x1d68a, digit: 0x1d7f6 }) },
  {
    id: "circled",
    label: "Circled",
    transform: rangeMapper({ upper: 0x24b6, lower: 0x24d0, digit: 0x2460 - 1, exceptions: { "0": "⓪" } }),
  },
  { id: "negativeCircled", label: "Black circled", transform: rangeMapper({ upper: 0x1f150, lower: 0x1f150 }) },
  { id: "squared", label: "Squared", transform: rangeMapper({ upper: 0x1f130, lower: 0x1f130 }) },
  { id: "negativeSquared", label: "Black squared", transform: rangeMapper({ upper: 0x1f170, lower: 0x1f170 }) },
  {
    id: "fullwidth",
    label: "Fullwidth",
    transform: rangeMapper({ upper: 0xff21, lower: 0xff41, digit: 0xff10 }),
  },
  {
    id: "smallCaps",
    label: "Small caps",
    transform: tableMapper(LOWER, "ᴀʙᴄᴅᴇꜰɢʜɪᴊᴋʟᴍɴᴏᴘǫʀsᴛᴜᴠᴡxʏᴢ"),
  },
  {
    id: "superscript",
    label: "Superscript",
    transform: tableMapper(
      `${LOWER}${DIGITS}+-=()`,
      "ᵃᵇᶜᵈᵉᶠᵍʰⁱʲᵏˡᵐⁿᵒᵖqʳˢᵗᵘᵛʷˣʸᶻ⁰¹²³⁴⁵⁶⁷⁸⁹⁺⁻⁼⁽⁾",
    ),
  },
  { id: "upsideDown", label: "Upside down", transform: upsideDown },
  { id: "strikethrough", label: "Strikethrough", transform: combining("̶") },
  { id: "underline", label: "Underline", transform: combining("̲") },
];

const BY_ID = new Map(FONT_STYLES.map((style) => [style.id, style]));

export function isFontStyleId(value: unknown): value is FontStyleId {
  return typeof value === "string" && BY_ID.has(value as FontStyleId);
}

/** Apply a font style to text; unknown/absent ids return the text unchanged. */
export function applyFontStyle(
  text: string,
  fontStyle: FontStyleId | null | undefined,
): string {
  if (!fontStyle) return text;
  return BY_ID.get(fontStyle)?.transform(text) ?? text;
}
