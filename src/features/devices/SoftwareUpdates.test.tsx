import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { getUpdateResult, checkForUpdates } = vi.hoisted(() => ({ getUpdateResult: vi.fn(), checkForUpdates: vi.fn() }));
vi.mock("@/lib/tauri/monitoring", () => ({ getUpdateResult, checkForUpdates }));
import { SoftwareUpdates } from "./SoftwareUpdates";

const result = { schemaVersion: 1, deviceId: "d", status: "updatesAvailable", checkedAt: "2026-08-12T13:30:00Z", support: { status: "supported" }, packageMetadata: { status: "stale", ageSeconds: 700000, staleAfterSeconds: 604800 }, updates: { totalCount: 2, truncated: true, packages: [{ name: "bash", installedVersion: "5.2", candidateVersion: "5.3" }, { name: "docker-ce", installedVersion: "1", candidateVersion: "2" }] }, keptBackPackages: { totalCount: 2, packages: ["linux-image", "firmware"], truncated: false }, heldPackages: { status: "known", totalCount: 1, packages: ["bash"], truncated: false }, reboot: "unknown", securityUpdates: { status: "unavailable" }, warnings: ["heldPackagesUnknown"], failure: { kind: "requiredCommand", message: "commandFailed" } };

describe("SoftwareUpdates", () => {
  beforeEach(() => { getUpdateResult.mockReset(); checkForUpdates.mockReset(); getUpdateResult.mockResolvedValue(result); });
  afterEach(cleanup);

  it("renders a concise System Updates summary without diagnostic details", async () => {
    render(<SoftwareUpdates deviceId="d" />);
    await screen.findByText("2 updates available");
    expect(screen.getByText("2 packages deferred")).toBeInTheDocument();
    expect(screen.getByText(/Last checked:/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Check again" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "View updates" })).toBeInTheDocument();
    expect(screen.queryByText(/700000s old/)).not.toBeInTheDocument();
    expect(screen.queryByText(/Held:/)).not.toBeInTheDocument();
    expect(screen.queryByText(/Reboot:/)).not.toBeInTheDocument();
    expect(screen.queryByText(/Partial data/)).not.toBeInTheDocument();
    expect(screen.queryByText(/commandFailed/)).not.toBeInTheDocument();
    expect(screen.queryByText(/bash \(5.2/)).not.toBeInTheDocument();
  });

  it("shows the bounded normal-update table only on request", async () => {
    render(<SoftwareUpdates deviceId="d" />);
    const action = await screen.findByRole("button", { name: "View updates" });
    expect(screen.queryByRole("table")).not.toBeInTheDocument();
    fireEvent.click(action);
    expect(screen.getByRole("table", { name: "Available system updates" })).toBeInTheDocument();
    expect(screen.getByText("bash")).toBeInTheDocument();
    expect(screen.getByText("docker-ce")).toBeInTheDocument();
    expect(screen.getByText("5.2")).toBeInTheDocument();
    expect(screen.getByText("5.3")).toBeInTheDocument();
    expect(screen.queryByText("linux-image")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Hide updates" })).toHaveAttribute("aria-expanded", "true");
  });

  it("does not present deferred-only results as zero updates", async () => {
    getUpdateResult.mockResolvedValue({ ...result, updates: { totalCount: 0, packages: [], truncated: false } });
    render(<SoftwareUpdates deviceId="d" />);
    await screen.findByText("System updates are available");
    expect(screen.getByText("2 packages deferred")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "View updates" })).not.toBeInTheDocument();
  });

  it("loads the cached result once and checks only after explicit action", async () => {
    checkForUpdates.mockResolvedValue({ ...result, status: "upToDate", updates: { totalCount: 0, packages: [], truncated: false }, keptBackPackages: { totalCount: 0, packages: [], truncated: false } });
    render(<SoftwareUpdates deviceId="d" />);
    await waitFor(() => expect(getUpdateResult).toHaveBeenCalledWith("d"));
    expect(checkForUpdates).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Check again" }));
    await waitFor(() => expect(checkForUpdates).toHaveBeenCalledWith("d"));
    expect(checkForUpdates).toHaveBeenCalledTimes(1);
  });

  it("shows the app spinner and restores the action after success", async () => {
    let resolveCheck!: (value: typeof result) => void;
    checkForUpdates.mockReturnValue(new Promise(resolve => { resolveCheck = resolve; }));
    render(<SoftwareUpdates deviceId="d" />);
    const button = await screen.findByRole("button", { name: "Check again" });
    fireEvent.click(button);
    expect(button).toBeDisabled();
    expect(screen.getByRole("button", { name: "Checking…" })).toBeDisabled();
    expect(button.querySelector("svg")).toHaveClass("animate-spin");
    expect(screen.getByRole("button", { name: "View updates" })).toBeDisabled();
    resolveCheck({ ...result, status: "upToDate", updates: { totalCount: 0, packages: [], truncated: false }, keptBackPackages: { totalCount: 0, packages: [], truncated: false } });
    await waitFor(() => expect(screen.getByRole("button", { name: "Check again" })).not.toBeDisabled());
  });

  it("restores the action after a failed check", async () => {
    checkForUpdates.mockRejectedValue(new Error("offline"));
    render(<SoftwareUpdates deviceId="d" />);
    const button = await screen.findByRole("button", { name: "Check again" });
    fireEvent.click(button);
    await waitFor(() => expect(screen.getByRole("button", { name: "Check for Updates" })).not.toBeDisabled());
    expect(screen.getByRole("button", { name: "Check for Updates" })).toBeInTheDocument();
    expect(checkForUpdates).toHaveBeenCalledTimes(1);
  });
});
