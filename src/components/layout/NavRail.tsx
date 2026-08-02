import { LayoutDashboard, Grid2x2, Settings } from "lucide-react";
import { cn } from "@/lib/utils";
import { useRouter, type Screen } from "@/app/router";

const NAV_ITEMS: {
  target: Screen["name"];
  label: string;
  icon: typeof LayoutDashboard;
}[] = [
  { target: "dashboard", label: "Dashboard", icon: LayoutDashboard },
  { target: "services", label: "Services", icon: Grid2x2 },
  { target: "globalSettings", label: "Settings", icon: Settings },
];

export function NavRail() {
  const { screen, goDashboard, goServices, goGlobalSettings } = useRouter();

  const handlers: Record<string, () => void> = {
    dashboard: goDashboard,
    services: () => goServices(),
    globalSettings: goGlobalSettings,
  };

  const isActive = (target: Screen["name"]) =>
    target === "globalSettings"
      ? screen.name === "globalSettings" || screen.name === "deviceSettings"
      : screen.name === target;

  return (
    <nav
      aria-label="Primary"
      className="flex w-16 shrink-0 flex-col items-center gap-1.5 border-r border-sidebar-border bg-sidebar py-3.5"
    >
      <div
        aria-hidden="true"
        className="mb-3 flex h-8 w-8 items-center justify-center rounded-lg bg-primary text-sm font-bold text-primary-foreground"
      >
        π
      </div>

      {NAV_ITEMS.map(({ target, label, icon: Icon }) => (
        <button
          key={target}
          type="button"
          title={label}
          aria-label={label}
          aria-current={isActive(target) ? "page" : undefined}
          onClick={handlers[target]}
          className={cn(
            "flex h-10 w-10 items-center justify-center rounded-lg text-sidebar-foreground transition-colors hover:bg-white/[0.08] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sidebar-ring",
            isActive(target) &&
              "bg-sidebar-accent text-sidebar-accent-foreground hover:bg-sidebar-accent",
          )}
        >
          <Icon className="h-[18px] w-[18px]" strokeWidth={2} />
        </button>
      ))}
    </nav>
  );
}
