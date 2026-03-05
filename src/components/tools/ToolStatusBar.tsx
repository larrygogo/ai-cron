import { useState, useEffect } from "react";
import * as api from "../../lib/tauri";
import type { ToolInfo } from "../../lib/types";

export function ToolStatusBar() {
  const [tools, setTools] = useState<ToolInfo[]>([]);

  useEffect(() => {
    api.detectTools().then(setTools).catch(() => {});
  }, []);

  if (tools.length === 0) return null;

  return (
    <div style={{ display: "flex", gap: 6, alignItems: "center", marginRight: 12 }}>
      {tools.map((t) => (
        <div
          key={t.name}
          title={`${t.label}: ${t.available ? `v${t.version ?? "?"}` : "未安装"}`}
          style={{ display: "flex", alignItems: "center", gap: 3 }}
        >
          <span
            style={{
              width: 5,
              height: 5,
              borderRadius: "50%",
              background: t.available ? "var(--accent)" : "var(--text-muted)",
              display: "inline-block",
            }}
          />
          <span
            style={{
              fontSize: 9,
              color: t.available ? "var(--text-secondary)" : "var(--text-muted)",
            }}
          >
            {t.name}
          </span>
        </div>
      ))}
    </div>
  );
}
