import { useEffect, useState } from "react";
import { ArrowLeft, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { DeviceForm } from "@/features/devices/DeviceForm";
import { useRouter } from "@/app/router";
import { getDevice } from "@/lib/tauri/devices";
import { clearDeviceThresholdOverrides, getAppSettings, saveDeviceThresholdOverrides } from "@/lib/tauri/settings";
import type { AppSettings, ThresholdPolicyOverrides } from "@/types/settings";
import type { Device } from "@/types/device";

interface DeviceSettingsScreenProps {
  deviceId: string;
}

export function DeviceSettingsScreen({ deviceId }: DeviceSettingsScreenProps) {
  const { goDashboard } = useRouter();
  const [device, setDevice] = useState<Device | null | undefined>(undefined);
  const [settings, setSettings] = useState<AppSettings | null>(null);

  useEffect(() => {
    let cancelled = false;
    // Fetch-on-mount: no data-fetching library is in scope for the MVP yet,
    // so the lint rule's suggested alternative (a query library) doesn't
    // apply here.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setDevice(undefined);
    getDevice(deviceId).then((result) => {
      if (!cancelled) {
        setDevice(result);
      }
    });
    getAppSettings().then(setSettings).catch(() => setSettings(null));
    return () => {
      cancelled = true;
    };
  }, [deviceId]);

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-2.5">
        <Button
          type="button"
          variant="outline"
          size="icon"
          onClick={goDashboard}
          aria-label="Back to dashboard"
        >
          <ArrowLeft />
        </Button>
        <h1 className="text-lg font-semibold text-foreground">
          {device ? `Settings · ${device.name}` : "Device settings"}
        </h1>
      </div>

      {device === undefined ? (
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="animate-spin" /> Loading…
        </div>
      ) : device === null ? (
        <p className="text-sm text-muted-foreground">
          This device no longer exists.
        </p>
      ) : (
        <>
          <DeviceForm
          mode="edit"
          device={device}
          onSaved={goDashboard}
          onCancel={goDashboard}
          />
          {settings ? <DeviceThresholdOverrides deviceId={device.id} settings={settings} onChanged={setSettings} /> : null}
        </>
      )}
    </div>
  );
}

function DeviceThresholdOverrides({ deviceId, settings, onChanged }: { deviceId: string; settings: AppSettings; onChanged: (value: AppSettings) => void }) {
  const current = settings.deviceThresholdOverrides[deviceId] ?? {};
  const [draft, setDraft] = useState<ThresholdPolicyOverrides>(current);
  const inherited = !settings.deviceThresholdOverrides[deviceId];
  async function save() { onChanged(await saveDeviceThresholdOverrides(deviceId, draft)); }
  async function clear() { onChanged(await clearDeviceThresholdOverrides(deviceId)); setDraft({}); }
  return <section className="max-w-xl rounded-lg border border-border bg-card p-4"><div className="mb-3 flex items-center justify-between"><div><h2 className="text-xs font-bold tracking-wide text-muted-foreground">ALERT THRESHOLDS</h2><p className="mt-1 text-xs text-muted-foreground">{inherited ? "Inherited from global defaults" : "Override active for this device"}</p></div>{!inherited ? <Button size="sm" variant="outline" onClick={clear}>Clear override</Button> : null}</div><div className="grid grid-cols-2 gap-3">{([['cpuWarningPercent','CPU warning (%)'],['cpuCriticalPercent','CPU critical (%)'],['memoryWarningPercent','Memory warning (%)'],['memoryCriticalPercent','Memory critical (%)'],['diskWarningPercent','Disk warning (%)'],['diskCriticalPercent','Disk critical (%)'],['temperatureWarningCelsius','Temperature warning (C)'],['temperatureCriticalCelsius','Temperature critical (C)'],['serviceUnavailableFailures','Service failures']] as const).map(([key, label]) => <div key={key} className="flex flex-col gap-1"><Label className="text-xs" htmlFor={`device-${key}`}>{label}</Label><Input id={`device-${key}`} type="number" placeholder={String(settings.thresholdPolicy[key])} value={draft[key] ?? ""} onChange={(event) => setDraft((value) => ({ ...value, [key]: event.target.value === "" ? undefined : Number(event.target.value) }))} /></div>)}</div><Button className="mt-3" size="sm" onClick={save}>Save override</Button></section>;
}
