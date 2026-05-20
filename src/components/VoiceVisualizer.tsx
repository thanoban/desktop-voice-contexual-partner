import { useEffect, useRef } from "react";
import { useChatStore } from "@/store/chatStore";

const BAR_COUNT = 20;

export function VoiceVisualizer() {
  const isListening = useChatStore((s) => s.isListening);
  const isSpeaking = useChatStore((s) => s.isSpeaking);
  const active = isListening || isSpeaking;
  const barsRef = useRef<(HTMLDivElement | null)[]>([]);
  const animRef = useRef<number>(0);
  const phaseRef = useRef<number[]>(
    Array.from({ length: BAR_COUNT }, () => Math.random() * Math.PI * 2)
  );

  useEffect(() => {
    if (!active) {
      barsRef.current.forEach((b) => {
        if (b) b.style.height = "3px";
      });
      return;
    }

    const color = isListening ? "var(--accent)" : "#3b82f6";

    const tick = (t: number) => {
      barsRef.current.forEach((b, i) => {
        if (!b) return;
        const phase = phaseRef.current[i];
        const freq = 0.0015 + i * 0.00008;
        const amp = 8 + Math.sin(i * 1.3 + t * 0.001) * 6;
        const h = Math.abs(Math.sin(t * freq + phase)) * amp + 3;
        b.style.height = `${h}px`;
        b.style.background = color;
      });
      animRef.current = requestAnimationFrame(tick);
    };

    animRef.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(animRef.current);
  }, [active, isListening]);

  return (
    <div
      style={{
        height: "28px",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        gap: "3px",
        opacity: active ? 1 : 0.15,
        transition: "opacity 0.3s ease",
      }}
    >
      {Array.from({ length: BAR_COUNT }).map((_, i) => (
        <div
          key={i}
          ref={(el) => { barsRef.current[i] = el; }}
          style={{
            width: "3px",
            height: "3px",
            borderRadius: "2px",
            background: "var(--text-muted)",
            transition: active ? "none" : "height 0.3s ease",
          }}
        />
      ))}
    </div>
  );
}
