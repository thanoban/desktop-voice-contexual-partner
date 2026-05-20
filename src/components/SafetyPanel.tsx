import { useState } from "react";

interface Props {
  visible: boolean;
  onDismiss: () => void;
}

const HELPLINES = [
  { region: "US",  name: "988 Suicide & Crisis Lifeline",                 number: "988",               url: "https://988lifeline.org" },
  { region: "UK",  name: "Samaritans",                                    number: "116 123",           url: "https://www.samaritans.org" },
  { region: "CA",  name: "Crisis Services Canada",                        number: "1-833-456-4566",    url: "https://www.crisisservicescanada.ca" },
  { region: "AU",  name: "Lifeline Australia",                            number: "13 11 14",          url: "https://www.lifeline.org.au" },
  { region: "LK",  name: "Sumithrayo Sri Lanka",                          number: "011-2696666",       url: "https://www.sumithrayo.org" },
  { region: "IN",  name: "iCall (India)",                                 number: "9152987821",        url: "https://icallhelpline.org" },
  { region: "NZ",  name: "Lifeline Aotearoa (NZ)",                        number: "0800 543 354",      url: "https://www.lifeline.org.nz" },
  { region: "ZA",  name: "SADAG (South Africa)",                          number: "0800 567 567",      url: "https://www.sadag.org" },
  { region: "INT", name: "International — IASP crisis centre directory",  number: "",                  url: "https://www.iasp.info/resources/Crisis_Centres" },
];

export function SafetyPanel({ visible, onDismiss }: Props) {
  const [region, setRegion] = useState("US");
  if (!visible) return null;

  const line = HELPLINES.find((h) => h.region === region) ?? HELPLINES[0];

  return (
    <div
      style={{
        position: "fixed",
        bottom: "60px",
        left: "50%",
        transform: "translateX(-50%)",
        zIndex: 200,
        width: "340px",
        background: "var(--bg-surface)",
        border: "1px solid var(--accent)",
        borderRadius: "12px",
        padding: "18px",
        boxShadow: "0 8px 32px rgba(0,0,0,0.6)",
      }}
      className="animate-fade-in"
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
        <p style={{ fontSize: "14px", fontWeight: 600, color: "var(--text-primary)" }}>
          You matter. Real support is available.
        </p>
        <button
          type="button"
          onClick={onDismiss}
          style={{ background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer", fontSize: "18px" }}
        >
          ✕
        </button>
      </div>

      <p style={{ fontSize: "13px", color: "var(--text-muted)", margin: "8px 0 12px" }}>
        I&apos;m an AI and I care about you — but a trained human can help more right now.
      </p>

      <select
        aria-label="Select your region"
        value={region}
        onChange={(e) => setRegion(e.target.value)}
        style={{
          background: "var(--bg-elevated)",
          border: "1px solid var(--text-dim)",
          borderRadius: "6px",
          color: "var(--text-primary)",
          padding: "5px 8px",
          fontSize: "12px",
          marginBottom: "10px",
          width: "100%",
          cursor: "pointer",
        }}
      >
        {HELPLINES.map((h) => (
          <option key={h.region} value={h.region}>{h.region} — {h.name}</option>
        ))}
      </select>

      <div
        style={{
          background: "rgba(167,139,250,0.08)",
          borderRadius: "8px",
          padding: "12px",
          display: "flex",
          flexDirection: "column",
          gap: "4px",
        }}
      >
        <span style={{ fontWeight: 600, fontSize: "14px" }}>{line.name}</span>
        {line.number && (
          <span style={{ fontSize: "20px", fontWeight: 700, color: "var(--accent)", letterSpacing: "0.04em" }}>
            {line.number}
          </span>
        )}
        <a
          href={line.url}
          target="_blank"
          rel="noopener noreferrer"
          style={{ fontSize: "12px", color: "var(--accent)", textDecoration: "none" }}
        >
          {line.url}
        </a>
      </div>
    </div>
  );
}
