import { getCurrentWindow } from "@tauri-apps/api/window";
import { Maximize2, Minus, X } from "lucide-react";
import { useConnectionStore } from "@/state/connectionStore";

const appWindow = getCurrentWindow();

const DOT: Record<string, string> = {
  Connected: "var(--color-status-connected)",
  Error: "var(--color-status-error)",
};

export function TitleBar() {
  const state = useConnectionStore((s) => s.status.state);
  const busy =
    state === "Launching" || state === "Connecting" || state === "Reconnecting" || state === "Disconnecting";
  return (
    // data-tauri-drag-region only fires when the mousedown target IS this
    // element, so the buttons stay clickable without any extra handling.
    <header
      data-tauri-drag-region
      className="relative z-10 flex h-9 shrink-0 select-none items-center justify-between"
    >
      <div data-tauri-drag-region className="flex h-full items-center gap-2 pl-3">
        <span
          aria-hidden
          className="size-1.5 rounded-full"
          style={{ backgroundColor: DOT[state] ?? (busy ? "var(--color-status-connecting)" : "var(--color-status-idle)") }}
        />
        <span className="text-[11px] font-medium tracking-wide text-muted-foreground">Aethery</span>
      </div>
      <div className="flex h-full items-stretch">
      <button
        aria-label="Minimize"
        className="grid h-full w-13 place-items-center text-muted-foreground hover:bg-surface-2 hover:text-foreground"
        onClick={() => void appWindow.minimize()}
      >
        <Minus className="size-4" />
      </button>
      <button
        aria-label="Maximize"
        className="grid h-full w-13 place-items-center text-muted-foreground hover:bg-surface-2 hover:text-foreground"
        onClick={() => void appWindow.toggleMaximize()}
      >
        <Maximize2 className="size-3.5" />
      </button>
      <button
        aria-label="Close"
        className="grid h-full w-13 place-items-center text-muted-foreground hover:bg-destructive hover:text-white"
        onClick={() => void appWindow.close()}
      >
        <X className="size-4" />
      </button>
      </div>
    </header>
  );
}
