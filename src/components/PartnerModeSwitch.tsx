import { useSettingsStore } from "@/store/settingsStore";

const modes = [
  { id: "company", label: "Company", title: "Warm conversation and shared presence" },
  { id: "work", label: "Work", title: "Practical help with concise spoken answers" },
  { id: "focus", label: "Focus", title: "Minimal, quiet assistance" },
] as const;

export function PartnerModeSwitch() {
  const selected = useSettingsStore((state) => state.settings.partner_mode);
  const update = useSettingsStore((state) => state.update);

  return (
    <div
      role="radiogroup"
      aria-label="Partner mode"
      style={{
        display: "flex",
        gap: "4px",
        padding: "7px 14px",
        background: "var(--bg-surface)",
        borderBottom: "1px solid var(--text-dim)",
      }}
    >
      {modes.map((mode) => {
        const active = selected === mode.id;
        return (
          <button
            key={mode.id}
            type="button"
            role="radio"
            aria-checked={active}
            title={mode.title}
            onClick={() => void update("partner_mode", mode.id)}
            style={{
              flex: 1,
              border: active ? "1px solid var(--accent)" : "1px solid transparent",
              borderRadius: "8px",
              background: active ? "var(--bg-elevated)" : "transparent",
              color: active ? "var(--text-primary)" : "var(--text-muted)",
              padding: "5px 8px",
              fontSize: "12px",
              fontWeight: active ? 600 : 400,
              cursor: "pointer",
            }}
          >
            {mode.label}
          </button>
        );
      })}
    </div>
  );
}
