import { useEffect, useMemo, useState } from "react";
import { Activity, Loader2 } from "lucide-react";
import { EmptyState } from "@/components/layout/EmptyState";
import { getActivity } from "@/lib/tauri/monitoring";
import { formatRelativeTime } from "@/lib/formatting/relativeTime";
import type { ActivityEvent, ActivityCategory } from "@/types/activity";

const categories: Array<ActivityCategory | "all"> = ["all", "device", "health", "service", "container", "diagnostic", "administration"];

export function ActivityScreen() {
  const [events, setEvents] = useState<ActivityEvent[] | null>(null);
  const [category, setCategory] = useState<ActivityCategory | "all">("all");
  const [deviceId, setDeviceId] = useState("all");
  const [error, setError] = useState<string | null>(null);
  useEffect(() => { getActivity().then(setEvents).catch(() => setError("Could not load activity history.")); }, []);
  const devices = useMemo(() => [...new Set((events ?? []).map((event) => event.deviceId).filter(Boolean))] as string[], [events]);
  const visible = (events ?? []).filter((event) => (category === "all" || event.category === category) && (deviceId === "all" || event.deviceId === deviceId));
  if (events === null && !error) return <div className="flex items-center justify-center gap-2 py-16 text-sm text-muted-foreground"><Loader2 className="animate-spin" /> Loading activity...</div>;
  return <div className="flex flex-col gap-4"><div className="flex flex-wrap items-start justify-between gap-3"><div><h1 className="text-xl font-semibold">Activity</h1><p className="mt-0.5 text-sm text-muted-foreground">Meaningful operational changes and completed actions.</p></div><div className="flex gap-2"><select value={category} onChange={(event) => setCategory(event.target.value as ActivityCategory | "all")} className="h-8 rounded-md border border-border bg-card px-2 text-sm">{categories.map((value) => <option key={value} value={value}>{value === "all" ? "All categories" : value}</option>)}</select><select value={deviceId} onChange={(event) => setDeviceId(event.target.value)} className="h-8 rounded-md border border-border bg-card px-2 text-sm"><option value="all">All devices</option>{devices.map((id) => <option key={id} value={id}>{id}</option>)}</select></div></div>{error ? <p className="text-sm text-destructive">{error}</p> : null}{visible.length === 0 ? <EmptyState icon={Activity} title="No activity yet" description="Meaningful device, service, container, and diagnostic events will appear here." /> : <div className="flex flex-col divide-y divide-border rounded-lg border border-border bg-card">{visible.map((event) => <div key={event.id} className="flex gap-3 p-3"><span className="mt-2 size-2 shrink-0 rounded-full bg-primary" /><div className="min-w-0"><p className="text-sm text-foreground">{event.summary}</p><p className="mt-1 text-xs text-muted-foreground">{event.category} {event.entityName ? `- ${event.entityName}` : ""} - {formatRelativeTime(event.timestamp)}</p></div></div>)}</div>}</div>;
}
