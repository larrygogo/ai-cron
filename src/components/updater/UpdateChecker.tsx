import { useState, useEffect } from "react";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export function UpdateChecker() {
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const [version, setVersion] = useState("");
  const [downloading, setDownloading] = useState(false);
  const [progress, setProgress] = useState("");

  useEffect(() => {
    checkForUpdate();
  }, []);

  async function checkForUpdate() {
    try {
      const update = await check();
      if (update) {
        setVersion(update.version);
        setUpdateAvailable(true);
      }
    } catch (e) {
      console.error("更新检查失败:", e);
    }
  }

  async function handleUpdate() {
    setDownloading(true);
    setProgress("正在下载更新...");
    try {
      const update = await check();
      if (!update) return;
      await update.downloadAndInstall((event) => {
        if (event.event === "Started" && event.data.contentLength) {
          setProgress(`正在下载 (${(event.data.contentLength / 1024 / 1024).toFixed(1)} MB)...`);
        } else if (event.event === "Finished") {
          setProgress("下载完成，正在安装...");
        }
      });
      await relaunch();
    } catch (e) {
      console.error("更新失败:", e);
      setProgress("更新失败，请稍后重试");
      setTimeout(() => {
        setDownloading(false);
        setProgress("");
      }, 3000);
    }
  }

  if (!updateAvailable) return null;

  return (
    <div
      style={{
        position: "fixed",
        bottom: 20,
        right: 20,
        background: "var(--bg-panel)",
        border: "1px solid var(--accent)",
        borderRadius: 8,
        padding: "14px 18px",
        zIndex: 9999,
        boxShadow: "0 4px 20px rgba(0,0,0,0.3)",
        maxWidth: 300,
      }}
    >
      <div
        style={{
          fontSize: 13,
          fontWeight: 600,
          color: "var(--text-primary)",
          marginBottom: 6,
        }}
      >
        发现新版本 v{version}
      </div>
      {downloading ? (
        <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
          {progress}
        </div>
      ) : (
        <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
          <button
            onClick={handleUpdate}
            style={{
              padding: "5px 14px",
              fontSize: 12,
              borderRadius: 4,
              border: "none",
              background: "var(--accent)",
              color: "#fff",
              cursor: "pointer",
              fontFamily: "inherit",
            }}
          >
            立即更新
          </button>
          <button
            onClick={() => setUpdateAvailable(false)}
            style={{
              padding: "5px 14px",
              fontSize: 12,
              borderRadius: 4,
              border: "1px solid var(--border)",
              background: "transparent",
              color: "var(--text-secondary)",
              cursor: "pointer",
              fontFamily: "inherit",
            }}
          >
            稍后
          </button>
        </div>
      )}
    </div>
  );
}
