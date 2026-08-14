import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { getUpdateResult, getMaintenanceOperation, checkForUpdates, prepareDeviceUpdate, applyPreparedDeviceUpdate, reconcileDeviceUpdate } = vi.hoisted(() => ({ getUpdateResult: vi.fn(), getMaintenanceOperation: vi.fn(), checkForUpdates: vi.fn(), prepareDeviceUpdate: vi.fn(), applyPreparedDeviceUpdate: vi.fn(), reconcileDeviceUpdate: vi.fn() }));
vi.mock("@/lib/tauri/monitoring", () => ({ getUpdateResult, getMaintenanceOperation, checkForUpdates, prepareDeviceUpdate, applyPreparedDeviceUpdate, reconcileDeviceUpdate }));
import { SoftwareUpdates } from "./SoftwareUpdates";

const result = { schemaVersion: 1, deviceId: "d", status: "updatesAvailable", checkedAt: "2026-08-12T13:30:00Z", support: { status: "supported" }, packageMetadata: { status: "stale", ageSeconds: 700000, staleAfterSeconds: 604800 }, updates: { totalCount: 2, truncated: true, packages: [{ name: "bash", installedVersion: "5.2", candidateVersion: "5.3" }, { name: "docker-ce", installedVersion: "1", candidateVersion: "2" }] }, keptBackPackages: { totalCount: 2, packages: ["linux-image", "firmware"], truncated: false }, heldPackages: { status: "known", totalCount: 1, packages: ["bash"], truncated: false }, reboot: "unknown", securityUpdates: { status: "unavailable" }, warnings: ["heldPackagesUnknown"], failure: { kind: "requiredCommand", message: "commandFailed" } };

describe("SoftwareUpdates", () => {
  beforeEach(() => { vi.useRealTimers(); getUpdateResult.mockReset(); getMaintenanceOperation.mockReset(); checkForUpdates.mockReset(); prepareDeviceUpdate.mockReset(); applyPreparedDeviceUpdate.mockReset(); reconcileDeviceUpdate.mockReset(); getUpdateResult.mockResolvedValue(result); getMaintenanceOperation.mockResolvedValue(null); });
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
    await waitFor(() => expect(screen.getByRole("button", { name: "Check again" })).not.toBeDisabled());
    expect(screen.getByText(/last known update information is still shown/i)).toBeInTheDocument();
    expect(checkForUpdates).toHaveBeenCalledTimes(1);
  });

  it("invokes Tauri for a terminal M16 device and preserves cached M15 evidence when the coordinator rejects the check", async () => {
    const deferred = { ...result, updates: { totalCount: 0, packages: [], truncated: false }, keptBackPackages: { totalCount: 5, packages: ["linux-image"], truncated: false }, failure: undefined, heldPackages: { ...result.heldPackages, status: "known" as const } };
    getUpdateResult.mockResolvedValue(deferred);
    getMaintenanceOperation.mockResolvedValue({ id: "complete", deviceId: "d", transientUnitId: "hidden", state: "completed", dispatchState: "accepted", requestedAt: "x", completedAt: "x" });
    checkForUpdates.mockRejectedValue({ code: "AlreadyChecking" });
    render(<SoftwareUpdates deviceId="d" />);
    const action = await screen.findByRole("button", { name: "Check again" });
    expect(action).not.toBeDisabled();
    fireEvent.click(action);
    await screen.findByText(/already in progress/i);
    expect(checkForUpdates).toHaveBeenCalledWith("d");
    expect(screen.getByText("5 packages deferred")).toBeInTheDocument();
    expect(screen.queryByText(/System updates have not been checked/i)).not.toBeInTheDocument();
  });

  it("only exposes Update device for complete safe evidence and does not mutate before final confirmation", async () => {
    const safe = { ...result, updates: { ...result.updates, truncated: false }, failure: undefined, heldPackages: { ...result.heldPackages, status: "known" as const } };
    getUpdateResult.mockResolvedValue(safe);
    prepareDeviceUpdate.mockResolvedValue({ plan: safe, operation: { id: "op", deviceId: "d", transientUnitId: "hidden", state: "requested", dispatchState: "notAttempted", requestedAt: "2026-08-12T00:00:00Z" } });
    render(<SoftwareUpdates deviceId="d" />);
    const update = await screen.findByRole("button", { name: "Update device" });
    fireEvent.click(update);
    expect(screen.getByRole("status")).toHaveTextContent("Preparing update…");
    await screen.findByRole("alertdialog");
    expect(screen.getByText(/Keep it powered on/i)).toBeInTheDocument();
    expect(applyPreparedDeviceUpdate).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(applyPreparedDeviceUpdate).not.toHaveBeenCalled();
  });

  it("closes confirmation to view packages and can return to the same prepared review without re-preparing", async () => {
    const safe = { ...result, updates: { ...result.updates, truncated: false }, failure: undefined, heldPackages: { ...result.heldPackages, status: "known" as const } };
    getUpdateResult.mockResolvedValue(safe);
    prepareDeviceUpdate.mockResolvedValue({ plan: safe, operation: { id: "op", deviceId: "d", transientUnitId: "hidden", state: "requested", dispatchState: "notAttempted", requestedAt: "x" } });
    render(<SoftwareUpdates deviceId="d" />);
    fireEvent.click(await screen.findByRole("button", { name: "Update device" }));
    await screen.findByRole("alertdialog");
    fireEvent.click(screen.getByRole("button", { name: "View packages" }));
    await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());
    expect(screen.getByRole("table", { name: "Available system updates" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Review prepared update" }));
    await screen.findByRole("alertdialog");
    expect(prepareDeviceUpdate).toHaveBeenCalledTimes(1);
  });

  it("does not reopen a persisted pre-dispatch operation after remount and instead requires a fresh prepare", async () => {
    const safe = { ...result, updates: { ...result.updates, truncated: false }, failure: undefined, heldPackages: { ...result.heldPackages, status: "known" as const } };
    getUpdateResult.mockResolvedValue(safe);
    getMaintenanceOperation.mockResolvedValue({ id: "old", deviceId: "d", transientUnitId: "hidden", state: "requested", dispatchState: "notAttempted", requestedAt: "x" });
    render(<SoftwareUpdates deviceId="d" />);
    await screen.findByRole("button", { name: "Update device" });
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("treats PlanChanged as a new review without dispatching again", async () => {
    const safe = { ...result, updates: { ...result.updates, truncated: false }, failure: undefined, heldPackages: { ...result.heldPackages, status: "known" as const } };
    getUpdateResult.mockResolvedValue(safe);
    prepareDeviceUpdate.mockResolvedValue({ plan: safe, operation: { id: "op", deviceId: "d", transientUnitId: "hidden", state: "requested", dispatchState: "notAttempted", requestedAt: "x" } });
    applyPreparedDeviceUpdate.mockResolvedValue({ id: "op", deviceId: "d", transientUnitId: "hidden", state: "planChanged", dispatchState: "notAttempted", requestedAt: "x", failure: "planChanged" });
    render(<SoftwareUpdates deviceId="d" />);
    fireEvent.click(await screen.findByRole("button", { name: "Update device" }));
    await screen.findByRole("alertdialog");
    fireEvent.click(screen.getByRole("button", { name: "Install 2 updates" }));
    await screen.findByText(/Available updates changed/i);
    expect(applyPreparedDeviceUpdate).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("button", { name: "Prepare updated plan" })).toBeInTheDocument();
  });

  it("maps typed privilege failures to an actionable message instead of a backend code", async () => {
    const safe = { ...result, updates: { ...result.updates, truncated: false }, failure: undefined, heldPackages: { ...result.heldPackages, status: "known" as const } };
    getUpdateResult.mockResolvedValue(safe);
    prepareDeviceUpdate.mockRejectedValue({ code: "PrivilegeUnavailable" });
    render(<SoftwareUpdates deviceId="d" />);
    fireEvent.click(await screen.findByRole("button", { name: "Update device" }));
    await screen.findByText(/passwordless sudo permission/i);
    expect(screen.queryByText("PrivilegeUnavailable")).not.toBeInTheDocument();
  });

  it("does not present a failed PREPARE transport check as an uncertain dispatched update", async () => {
    const safe = { ...result, updates: { ...result.updates, truncated: false }, failure: undefined, heldPackages: { ...result.heldPackages, status: "known" as const } };
    getUpdateResult.mockResolvedValue(safe);
    prepareDeviceUpdate.mockRejectedValue({ code: "TransportUnavailableDuringObservation" });
    render(<SoftwareUpdates deviceId="d" />);
    fireEvent.click(await screen.findByRole("button", { name: "Update device" }));
    await screen.findByText(/No packages were changed/i);
    expect(screen.queryByText(/outcome is uncertain/i)).not.toBeInTheDocument();
  });

  it("renders a persisted pre-dispatch transport failure as safe to retry, not uncertain", async () => {
    const safe = { ...result, updates: { ...result.updates, truncated: false }, failure: undefined, heldPackages: { ...result.heldPackages, status: "known" as const } };
    getUpdateResult.mockResolvedValue(safe);
    getMaintenanceOperation.mockResolvedValue({ id: "op", deviceId: "d", transientUnitId: "hidden", state: "preDispatchFailed", dispatchState: "notAttempted", requestedAt: "x", completedAt: "x", failure: "transportUnavailableDuringObservation", preDispatchFailureStage: "planVerification" });
    render(<SoftwareUpdates deviceId="d" />);
    await screen.findByText(/Update did not start/i);
    expect(screen.getByText(/No packages were changed. You can try again/i)).toBeInTheDocument();
    expect(screen.queryByText(/outcome is uncertain/i)).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Update device" })).toBeInTheDocument();
    expect(reconcileDeviceUpdate).not.toHaveBeenCalled();
  });

  it("reconciles one at a time after five seconds and keeps a transient observation error nonterminal", async () => {
    vi.useFakeTimers();
    const active = { id: "op", deviceId: "d", transientUnitId: "hidden", state: "installing", dispatchState: "accepted", requestedAt: "x", observationDeadline: "2099-01-01T00:00:00Z" };
    getMaintenanceOperation.mockResolvedValue(active);
    reconcileDeviceUpdate.mockRejectedValue(new Error("ssh lost"));
    render(<SoftwareUpdates deviceId="d" />);
    await vi.advanceTimersByTimeAsync(0);
    await vi.advanceTimersByTimeAsync(5000);
    expect(reconcileDeviceUpdate).toHaveBeenCalledTimes(1);
    expect(screen.getByText(/may still be continuing/i)).toBeInTheDocument();
    expect(screen.queryByText(/could not be completed safely/i)).not.toBeInTheDocument();
  });

  it("stops automatic reconciliation after the persisted deadline but permits Check status", async () => {
    vi.useFakeTimers();
    getMaintenanceOperation.mockResolvedValue({ id: "op", deviceId: "d", transientUnitId: "hidden", state: "stillRunning", dispatchState: "accepted", requestedAt: "x", observationDeadline: "2000-01-01T00:00:00Z" });
    reconcileDeviceUpdate.mockResolvedValue({ id: "op", deviceId: "d", transientUnitId: "hidden", state: "stillRunning", dispatchState: "accepted", requestedAt: "x", observationDeadline: "2000-01-01T00:00:00Z" });
    render(<SoftwareUpdates deviceId="d" />);
    await vi.advanceTimersByTimeAsync(0);
    expect(screen.getByRole("button", { name: "Check status" })).toBeInTheDocument();
    await vi.advanceTimersByTimeAsync(10000);
    expect(reconcileDeviceUpdate).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Check status" }));
    await vi.advanceTimersByTimeAsync(0);
    expect(reconcileDeviceUpdate).toHaveBeenCalledTimes(1);
  });

  it("renders uncertain and verified terminal outcomes without internal identifiers", async () => {
    getMaintenanceOperation.mockResolvedValue({ id: "op", deviceId: "d", transientUnitId: "pihub-update-secret.service", state: "outcomeUncertain", dispatchState: "uncertain", requestedAt: "x", observationDeadline: "2000-01-01T00:00:00Z" });
    const { rerender } = render(<SoftwareUpdates deviceId="d" />);
    await screen.findByText(/may have started/i);
    expect(screen.queryByText(/pihub-update-secret/i)).not.toBeInTheDocument();
    getMaintenanceOperation.mockResolvedValue({ id: "op", deviceId: "d", transientUnitId: "hidden", state: "completedRebootRequired", dispatchState: "accepted", requestedAt: "x", packageCount: 2 });
    rerender(<SoftwareUpdates deviceId="next" />);
    await screen.findByText(/restart required/i);
  });

  it("uses persisted terminal verification results without starting another M15 check", async () => {
    getMaintenanceOperation.mockResolvedValue({ id: "op", deviceId: "d", transientUnitId: "hidden", state: "verifying", dispatchState: "accepted", requestedAt: "x", observationDeadline: "2099-01-01T00:00:00Z" });
    reconcileDeviceUpdate.mockResolvedValue({ id: "op", deviceId: "d", transientUnitId: "hidden", state: "completed", dispatchState: "accepted", requestedAt: "x", packageCount: 2 });
    vi.useFakeTimers();
    render(<SoftwareUpdates deviceId="d" />);
    await vi.advanceTimersByTimeAsync(0);
    await vi.advanceTimersByTimeAsync(5000);
    expect(screen.getByText(/Update complete/i)).toBeInTheDocument();
    expect(checkForUpdates).not.toHaveBeenCalled();
  });
});
