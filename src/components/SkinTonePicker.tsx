import { SKIN_TONES } from "../types/preferences";
import type { SkinTone } from "../data/loadEmojis";

type SkinTonePickerProps = {
  skinTone: SkinTone;
  onSkinTone: (tone: SkinTone) => void;
};

export function SkinTonePicker({ skinTone, onSkinTone }: SkinTonePickerProps) {
  return (
    <div className="skin-tones" role="group" aria-labelledby="skin-tone-label" aria-label="Skin tone">
      {SKIN_TONES.map((tone) => (
        <button
          key={tone.tone}
          type="button"
          className="skin-swatch"
          style={{ background: tone.swatch }}
          title={tone.label}
          aria-label={tone.label}
          aria-pressed={skinTone === tone.tone}
          onClick={() => onSkinTone(tone.tone)}
        />
      ))}
    </div>
  );
}
