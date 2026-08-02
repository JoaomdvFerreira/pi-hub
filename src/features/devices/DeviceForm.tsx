import { useState, type FormEvent } from "react";
import { Loader2, Wifi } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { ServicesEditor } from "@/features/devices/ServicesEditor";
import { createDevice, testDeviceConnection, updateDevice } from "@/lib/tauri/devices";
import {
  connectionStatusColorClass,
  connectionStatusLabel,
} from "@/lib/formatting/connectionStatus";
import type { ApplicationError } from "@/types/settings";
import type {
  ConnectionTestResult,
  Device,
  DeviceInput,
  DeviceType,
} from "@/types/device";

const DEVICE_TYPE_OPTIONS: { value: DeviceType; label: string }[] = [
  { value: "raspberry-pi", label: "Raspberry Pi" },
  { value: "linux-server", label: "Linux server" },
  { value: "mini-pc", label: "Mini PC" },
  { value: "nas", label: "NAS" },
  { value: "other", label: "Other" },
];

const REFRESH_INTERVAL_OPTIONS: { value: string; label: string }[] = [
  { value: "default", label: "Use global default" },
  { value: "15", label: "15 seconds" },
  { value: "30", label: "30 seconds" },
  { value: "60", label: "1 minute" },
  { value: "300", label: "5 minutes" },
];

function toFormState(device?: Device): DeviceInput {
  return {
    name: device?.name ?? "",
    host: device?.host ?? "",
    sshPort: device?.sshPort ?? 22,
    sshUsername: device?.sshUsername ?? "",
    description: device?.description ?? "",
    deviceType: device?.deviceType ?? "raspberry-pi",
    monitoringEnabled: device?.monitoringEnabled ?? true,
    refreshIntervalSeconds: device?.refreshIntervalSeconds,
    notificationsEnabled: device?.notificationsEnabled ?? true,
    services: device?.services ?? [],
  };
}

function isApplicationError(err: unknown): err is ApplicationError {
  return (
    typeof err === "object" &&
    err !== null &&
    "code" in err &&
    "message" in err
  );
}

type TestState =
  | { kind: "idle" }
  | { kind: "testing" }
  | { kind: "result"; result: ConnectionTestResult };

interface DeviceFormProps {
  mode: "create" | "edit";
  device?: Device;
  onSaved: (device: Device) => void;
  onCancel: () => void;
}

export function DeviceForm({ mode, device, onSaved, onCancel }: DeviceFormProps) {
  const [form, setForm] = useState<DeviceInput>(() => toFormState(device));
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);
  const [testState, setTestState] = useState<TestState>({ kind: "idle" });

  function update<K extends keyof DeviceInput>(key: K, value: DeviceInput[K]) {
    setForm((prev) => ({ ...prev, [key]: value }));
  }

  async function handleTestConnection() {
    setTestState({ kind: "testing" });
    try {
      const result = await testDeviceConnection({
        host: form.host,
        sshPort: form.sshPort,
        sshUsername: form.sshUsername,
      });
      setTestState({ kind: "result", result });
    } catch (err) {
      const message = isApplicationError(err)
        ? err.message
        : "The connection test could not run.";
      setTestState({ kind: "result", result: { status: "unknown", message } });
    }
  }

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    setSaving(true);
    setFormError(null);
    try {
      const saved =
        mode === "create"
          ? await createDevice(form)
          : await updateDevice(device!.id, form);
      onSaved(saved);
    } catch (err) {
      setFormError(
        isApplicationError(err) ? err.message : "Failed to save the device.",
      );
    } finally {
      setSaving(false);
    }
  }

  return (
    <form onSubmit={handleSubmit} className="flex max-w-xl flex-col gap-4">
      <div className="flex flex-col gap-3 rounded-lg border border-border bg-card p-4">
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="device-name">Name</Label>
          <Input
            id="device-name"
            value={form.name}
            onChange={(e) => update("name", e.target.value)}
            required
          />
        </div>

        <div className="flex flex-col gap-1.5">
          <Label htmlFor="device-host">Hostname</Label>
          <Input
            id="device-host"
            className="font-mono"
            value={form.host}
            onChange={(e) => update("host", e.target.value)}
            placeholder="raspberrypi5.tail3f2a.ts.net"
            required
          />
        </div>

        <div className="grid grid-cols-2 gap-3">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="device-ssh-user">SSH username</Label>
            <Input
              id="device-ssh-user"
              className="font-mono"
              value={form.sshUsername}
              onChange={(e) => update("sshUsername", e.target.value)}
              required
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="device-ssh-port">SSH port</Label>
            <Input
              id="device-ssh-port"
              type="number"
              min={1}
              max={65535}
              value={form.sshPort}
              onChange={(e) => update("sshPort", Number(e.target.value))}
              required
            />
          </div>
        </div>

        <div className="flex flex-col gap-1.5">
          <Label htmlFor="device-description">Description</Label>
          <Textarea
            id="device-description"
            rows={2}
            value={form.description ?? ""}
            onChange={(e) => update("description", e.target.value)}
          />
        </div>

        <div className="grid grid-cols-2 gap-3">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="device-type">Device type</Label>
            <Select
              value={form.deviceType}
              onValueChange={(value) => update("deviceType", value as DeviceType)}
            >
              <SelectTrigger id="device-type" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {DEVICE_TYPE_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="device-refresh-interval">Refresh interval</Label>
            <Select
              value={
                form.refreshIntervalSeconds
                  ? String(form.refreshIntervalSeconds)
                  : "default"
              }
              onValueChange={(value) =>
                update(
                  "refreshIntervalSeconds",
                  value === "default" ? undefined : Number(value),
                )
              }
            >
              <SelectTrigger id="device-refresh-interval" className="w-full">
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
        </div>
      </div>

      <div className="flex flex-col gap-3 rounded-lg border border-border bg-card p-4">
        <div className="flex items-center justify-between">
          <Label htmlFor="device-monitoring">Monitoring enabled</Label>
          <Switch
            id="device-monitoring"
            checked={form.monitoringEnabled}
            onCheckedChange={(checked) => update("monitoringEnabled", checked)}
          />
        </div>
        <div className="flex items-center justify-between">
          <Label htmlFor="device-notifications">Notifications enabled</Label>
          <Switch
            id="device-notifications"
            checked={form.notificationsEnabled}
            onCheckedChange={(checked) => update("notificationsEnabled", checked)}
          />
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-3 rounded-lg border border-border bg-card p-4">
        <Button
          type="button"
          variant="outline"
          onClick={handleTestConnection}
          disabled={testState.kind === "testing" || !form.host || !form.sshUsername}
        >
          {testState.kind === "testing" ? (
            <Loader2 className="animate-spin" />
          ) : (
            <Wifi />
          )}
          Test Connection
        </Button>
        {testState.kind === "result" ? (
          <div className="flex flex-col">
            <span
              className={`text-sm font-medium ${connectionStatusColorClass(testState.result.status)}`}
            >
              {connectionStatusLabel(testState.result.status)}
            </span>
            {testState.result.message ? (
              <span className="text-xs text-muted-foreground">
                {testState.result.message}
              </span>
            ) : null}
          </div>
        ) : null}
      </div>

      <div className="flex flex-col gap-3 rounded-lg border border-border bg-card p-4">
        <h2 className="text-xs font-bold tracking-wide text-muted-foreground">
          REGISTERED SERVICES
        </h2>
        <ServicesEditor
          services={form.services}
          onChange={(services) => update("services", services)}
        />
      </div>

      {formError ? <p className="text-sm text-destructive">{formError}</p> : null}

      <div className="flex justify-end gap-2">
        <Button type="button" variant="outline" onClick={onCancel}>
          Cancel
        </Button>
        <Button type="submit" disabled={saving}>
          {saving ? <Loader2 className="animate-spin" /> : null}
          {mode === "create" ? "Add device" : "Save changes"}
        </Button>
      </div>
    </form>
  );
}
