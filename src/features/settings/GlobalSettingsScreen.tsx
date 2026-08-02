import { useCallback, useEffect, useState } from "react";
import { Check, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";
import { getAppSettings, saveAppSettings } from "@/lib/tauri/settings";
import type { AppSettings, ApplicationError, Theme } from "@/types/settings";
import pkg from "../../../package.json";

const REFRESH_INTERVAL_OPTIONS: { value: string; label: string }[] = [
  { value: "15", label: "15 seconds" },
  { value: "30", label: "30 seconds" },
  { value: "60", label: "1 minute" },
  { value: "300", label: "5 minutes" },
];

const THEME_OPTIONS: { value: Theme; label: string }[] = [
  { value: "dark", label: "Dark" },
  { value: "light", label: "Light" },
  { value: "system", label: "System" },
];

function isApplicationError(err: unknown): err is ApplicationError {
  return typeof err === "object" && err !== null && "code" in err && "message" in err;
}

type SaveState = "idle" | "saving" | "saved" | "error";

export function GlobalSettingsScreen() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [saveState, setSaveState] = useState<SaveState>("idle");
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const result = await getAppSettings();
      setSettings(result);
    } catch {
      setError("Could not load settings.");
    }
  }, []);

  useEffect(() => {
    // Fetch-on-mount: no data-fetching library is in scope for the MVP yet.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    load();
  }, [load]);

  async function persist(next: AppSettings) {
    setSettings(next);
    setSaveState("saving");
    setError(null);
    try {
      const saved = await saveAppSettings(next);
      setSettings(saved);
      setSaveState("saved");
    } catch (err) {
      setSaveState("error");
      setError(isApplicationError(err) ? err.message : "Could not save settings.");
    }
  }

  if (settings === null) {
    return (
      <div className="flex items-center justify-center gap-2 rounded-lg border border-dashed border-border py-16 text-sm text-muted-foreground">
        <Loader2 className="animate-spin" /> Loading…
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold text-foreground">Settings</h1>
          <p className="text-sm text-muted-foreground">General application preferences</p>
        </div>
        <div className="flex h-5 items-center gap-1.5 text-xs text-muted-foreground">
          {saveState === "saving" ? (
            <>
              <Loader2 className="size-3.5 animate-spin" /> Saving…
            </>
          ) : saveState === "saved" ? (
            <>
              <Check className="size-3.5 text-status-healthy" /> Saved
            </>
          ) : null}
        </div>
      </div>

      <div className="mt-4 flex max-w-xl flex-col gap-3.5">
        <section className="rounded-lg border border-border bg-card p-4">
          <h2 className="mb-2.5 text-xs font-bold tracking-wide text-muted-foreground">
            APPEARANCE
          </h2>
          <div className="flex items-center justify-between">
            <span className="text-sm text-foreground">Theme</span>
            <div className="flex gap-0.5 rounded-md bg-input/40 p-0.5">
              {THEME_OPTIONS.map((opt) => (
                <button
                  key={opt.value}
                  type="button"
                  onClick={() => persist({ ...settings, theme: opt.value })}
                  className={cn(
                    "rounded px-3 py-1.5 text-xs font-semibold transition-colors",
                    settings.theme === opt.value
                      ? "bg-primary text-primary-foreground"
                      : "text-muted-foreground hover:text-foreground",
                  )}
                >
                  {opt.label}
                </button>
              ))}
            </div>
          </div>
        </section>

        <section className="rounded-lg border border-border bg-card p-4">
          <h2 className="mb-2.5 text-xs font-bold tracking-wide text-muted-foreground">
            STARTUP
          </h2>
          <div className="flex items-center justify-between gap-4">
            <div>
              <Label htmlFor="start-with-windows" className="text-sm font-normal text-foreground">
                Start Pi-Hub with Windows
              </Label>
              <p className="mt-0.5 text-xs text-muted-foreground">
                Launches minimized to the system tray.
              </p>
            </div>
            <Switch
              id="start-with-windows"
              checked={settings.startWithWindows}
              onCheckedChange={(checked) =>
                persist({ ...settings, startWithWindows: checked })
              }
            />
          </div>
        </section>

        <section className="rounded-lg border border-border bg-card p-4">
          <h2 className="mb-2.5 text-xs font-bold tracking-wide text-muted-foreground">
            MONITORING
          </h2>
          <div className="flex items-center justify-between">
            <span className="text-sm text-foreground">Default refresh interval</span>
            <Select
              value={String(settings.refreshIntervalSeconds)}
              onValueChange={(value) =>
                persist({ ...settings, refreshIntervalSeconds: Number(value) })
              }
            >
              <SelectTrigger className="w-[150px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {REFRESH_INTERVAL_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </section>

        <section className="rounded-lg border border-border bg-card p-4">
          <h2 className="mb-2.5 text-xs font-bold tracking-wide text-muted-foreground">
            NOTIFICATIONS
          </h2>
          <div className="flex items-center justify-between">
            <Label htmlFor="global-notifications" className="text-sm font-normal text-foreground">
              Enable desktop notifications
            </Label>
            <Switch
              id="global-notifications"
              checked={settings.notificationsEnabled}
              onCheckedChange={(checked) =>
                persist({ ...settings, notificationsEnabled: checked })
              }
            />
          </div>
        </section>

        <section className="rounded-lg border border-border bg-card p-4">
          <h2 className="mb-3 text-xs font-bold tracking-wide text-muted-foreground">ABOUT</h2>
          <div className="flex items-center gap-3">
            <span className="flex size-[38px] shrink-0 items-center justify-center rounded-md bg-primary text-base font-bold text-primary-foreground">
              π
            </span>
            <div>
              <div className="text-sm font-semibold text-foreground">Pi-Hub</div>
              <div className="text-xs text-muted-foreground">Version {pkg.version}</div>
            </div>
            <div className="flex-1" />
            <Button variant="outline" size="sm" disabled title="Not available in this build">
              Check for updates
            </Button>
          </div>
        </section>

        {error ? <p className="text-sm text-destructive">{error}</p> : null}
      </div>
    </div>
  );
}
