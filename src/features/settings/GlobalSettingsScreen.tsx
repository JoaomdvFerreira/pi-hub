export function GlobalSettingsScreen() {
  return (
    <div className="flex flex-col gap-1">
      <h1 className="text-xl font-semibold text-foreground">Settings</h1>
      <p className="mb-4 text-sm text-muted-foreground">
        General application preferences
      </p>
      <div className="max-w-xl rounded-lg border border-border bg-card p-6 text-sm text-muted-foreground">
        Appearance, startup, monitoring, and notification preferences will
        appear here.
      </div>
    </div>
  );
}
