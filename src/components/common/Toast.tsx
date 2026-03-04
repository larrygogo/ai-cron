import { create } from "zustand";
import { useEffect } from "react";

interface ToastState {
  message: string | null;
  type: "success" | "error" | "info";
  show: (msg: string, type?: "success" | "error" | "info") => void;
  clear: () => void;
}

export const useToast = create<ToastState>((set) => ({
  message: null,
  type: "info",
  show: (msg, type = "info") => {
    set({ message: msg, type });
  },
  clear: () => set({ message: null }),
}));

const typeColors: Record<string, string> = {
  success: "var(--accent)",
  error: "var(--accent-red)",
  info: "var(--accent-blue)",
};

export function Toast() {
  const { message, type, clear } = useToast();

  useEffect(() => {
    if (message) {
      const timer = setTimeout(clear, 3000);
      return () => clearTimeout(timer);
    }
  }, [message, clear]);

  if (!message) return null;

  return (
    <div
      style={{
        position: "fixed",
        bottom: 16,
        right: 16,
        zIndex: 200,
        background: "var(--bg-panel)",
        border: `1px solid ${typeColors[type] ?? "var(--border)"}`,
        borderRadius: 6,
        padding: "8px 14px",
        fontSize: 12,
        color: "var(--text-primary)",
        boxShadow: "0 8px 24px rgba(0,0,0,0.4)",
        maxWidth: 360,
        display: "flex",
        alignItems: "center",
        gap: 8,
        animation: "fadeIn 0.15s ease-out",
      }}
    >
      <span
        style={{
          width: 6,
          height: 6,
          borderRadius: "50%",
          background: typeColors[type],
          flexShrink: 0,
        }}
      />
      <span style={{ flex: 1 }}>{message}</span>
      <button
        onClick={clear}
        style={{
          background: "none",
          border: "none",
          color: "var(--text-muted)",
          cursor: "pointer",
          fontSize: 14,
          padding: 0,
          lineHeight: 1,
        }}
      >
        ×
      </button>
    </div>
  );
}
