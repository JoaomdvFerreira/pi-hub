import { useState } from "react";
import { Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import type { DeviceService } from "@/types/device";

function isHttpOrHttpsUrl(raw: string): boolean {
  try {
    const parsed = new URL(raw);
    return parsed.protocol === "http:" || parsed.protocol === "https:";
  } catch {
    return false;
  }
}

interface ServicesEditorProps {
  services: DeviceService[];
  onChange: (services: DeviceService[]) => void;
}

export function ServicesEditor({ services, onChange }: ServicesEditorProps) {
  const [name, setName] = useState("");
  const [url, setUrl] = useState("");
  const [error, setError] = useState<string | null>(null);

  function handleAdd() {
    if (name.trim() === "") {
      setError("Give the service a name.");
      return;
    }
    if (!isHttpOrHttpsUrl(url)) {
      setError("Service URL must be a valid http:// or https:// address.");
      return;
    }
    setError(null);
    const service: DeviceService = {
      id: crypto.randomUUID(),
      name: name.trim(),
      url,
      enabled: true,
    };
    onChange([...services, service]);
    setName("");
    setUrl("");
  }

  function handleRemove(id: string) {
    onChange(services.filter((s) => s.id !== id));
  }

  function handleToggle(id: string, enabled: boolean) {
    onChange(services.map((s) => (s.id === id ? { ...s, enabled } : s)));
  }

  return (
    <div className="flex flex-col gap-3">
      {services.length === 0 ? (
        <p className="text-sm text-muted-foreground">
          No services registered on this device yet.
        </p>
      ) : (
        <div className="flex flex-col gap-2">
          {services.map((svc) => (
            <div
              key={svc.id}
              className="flex items-center gap-2.5 rounded-md border border-border px-2.5 py-2"
            >
              <div className="min-w-0 flex-1">
                <div className="truncate text-sm font-medium text-foreground">{svc.name}</div>
                <div className="truncate font-mono text-xs text-muted-foreground">{svc.url}</div>
              </div>
              <Switch
                checked={svc.enabled}
                onCheckedChange={(checked) => handleToggle(svc.id, checked)}
                aria-label={`Enable ${svc.name}`}
              />
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="size-7 text-muted-foreground"
                onClick={() => handleRemove(svc.id)}
                aria-label={`Remove ${svc.name}`}
              >
                <Trash2 className="size-3.5" />
              </Button>
            </div>
          ))}
        </div>
      )}

      <div className="flex flex-col gap-2 border-t border-border pt-3">
        <div className="grid grid-cols-[1fr_1.4fr_auto] items-end gap-2">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="service-name" className="text-xs">
              Name
            </Label>
            <Input
              id="service-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Home Assistant"
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="service-url" className="text-xs">
              URL
            </Label>
            <Input
              id="service-url"
              className="font-mono"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="http://raspberrypi5:8123"
            />
          </div>
          <Button type="button" variant="outline" onClick={handleAdd}>
            <Plus /> Add
          </Button>
        </div>
        {error ? <p className="text-xs text-destructive">{error}</p> : null}
      </div>
    </div>
  );
}
