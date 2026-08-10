import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DiagnosticsList, HealthDetails, HealthSummary, PowerThrottling } from "./HealthDiagnostics";
import type { ConnectivityDiagnosticReport, DeviceHealthAssessment } from "@/types/snapshot";

function health(state: DeviceHealthAssessment["state"], reasons: DeviceHealthAssessment["reasons"] = []): DeviceHealthAssessment {
  return { state, reasons, power: {} };
}

describe("M6 health presentation", () => {
  it.each(["healthy", "warning", "critical", "unknown"] as const)("renders %s with accessible status text", (state) => {
    render(<HealthDetails health={health(state)} />);
    expect(screen.getByText(state)).toBeInTheDocument();
  });

  it("shows the primary dashboard reason without reclassifying health", () => {
    render(<HealthSummary health={health("warning", [{ code: "disk_warning", severity: "warning", summary: "Root disk usage is high" }])} />);
    expect(screen.getByLabelText("Health: warning")).toHaveTextContent("Root disk usage is high");
  });

  it("keeps unavailable power values distinct from clear values", () => {
    render(<PowerThrottling health={{ ...health("unknown"), power: { undervoltageNow: true, undervoltageSinceBoot: false } }} />);
    expect(screen.getByText("Current: Detected · Since boot: Clear")).toBeInTheDocument();
    expect(screen.getAllByText(/Current: Unavailable/)).toHaveLength(3);
    expect(screen.getByText("Raw: Unavailable")).toBeInTheDocument();
  });
});

describe("M6 diagnostics presentation", () => {
  it("renders passed, warning, failed, and skipped statuses with stage labels", () => {
    const diagnostics: ConnectivityDiagnosticReport = { durationMs: 12, checks: [
      { code: "target_resolution", status: "passed", summary: "Target resolved" },
      { code: "docker", status: "warning", summary: "Docker is unavailable" },
      { code: "ssh_host_key", status: "failed", summary: "SSH host key verification failed" },
      { code: "tailscale", status: "skipped", summary: "Skipped because an earlier connection stage failed" },
    ] };
    render(<DiagnosticsList diagnostics={diagnostics} />);
    expect(screen.getByLabelText("target_resolution: passed")).toBeInTheDocument();
    expect(screen.getByLabelText("docker: warning")).toBeInTheDocument();
    expect(screen.getByLabelText("ssh_host_key: failed")).toBeInTheDocument();
    expect(screen.getByLabelText("tailscale: skipped")).toBeInTheDocument();
  });
});
