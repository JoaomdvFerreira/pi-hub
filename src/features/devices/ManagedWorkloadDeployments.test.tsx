import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { listManagedWorkloadDeployments, prepareManagedWorkloadDeployment, continueManagedWorkloadDeployment, reconcileManagedWorkloadDeployment } = vi.hoisted(() => ({ listManagedWorkloadDeployments: vi.fn(), prepareManagedWorkloadDeployment: vi.fn(), continueManagedWorkloadDeployment: vi.fn(), reconcileManagedWorkloadDeployment: vi.fn() }));
vi.mock("@/lib/tauri/managedWorkloads", () => ({ listManagedWorkloadDeployments, prepareManagedWorkloadDeployment, continueManagedWorkloadDeployment, reconcileManagedWorkloadDeployment }));
import { ManagedWorkloadDeployments } from "./ManagedWorkloadDeployments";

const card = { workloadId: "finance", name: "Personal Finance", enabled: true, eligibleToPrepare: true };
const prepared = { operation: { operation: { workloadId: "finance", operationId: "opaque-operation" }, state: "prepared" }, reviewTargetRevision: "a".repeat(40), changeCount: 2 };
const operation = { workloadId: "finance", operationId: "opaque-operation" };
function persisted(state: string, observationDeadline?: string) { return { ...card, eligibleToPrepare: !["dispatching", "dispatchUncertain", "deploying", "stillRunning", "awaitingVerification", "revisionVerified", "workloadRuntimeVerified"].includes(state), deployment: { operation, state, observationDeadline } }; }

describe("ManagedWorkloadDeployments", () => {
  beforeEach(() => { listManagedWorkloadDeployments.mockReset().mockResolvedValue([card]); prepareManagedWorkloadDeployment.mockReset().mockResolvedValue(prepared); continueManagedWorkloadDeployment.mockReset(); reconcileManagedWorkloadDeployment.mockReset(); });
  afterEach(() => { vi.useRealTimers(); cleanup(); });
  async function openReview() { render(<ManagedWorkloadDeployments deviceId="pi5"/>); fireEvent.click(await screen.findByRole("button", { name: "Prepare update" })); await screen.findByRole("alertdialog"); }

  it("renders configured workloads and prevents duplicate PREPARE", async () => {
    let resolve!: (value: typeof prepared) => void; prepareManagedWorkloadDeployment.mockReturnValue(new Promise(value => { resolve = value; }));
    render(<ManagedWorkloadDeployments deviceId="pi5"/>); const action = await screen.findByRole("button", { name: "Prepare update" });
    fireEvent.click(action); fireEvent.click(action); expect(prepareManagedWorkloadDeployment).toHaveBeenCalledTimes(1); expect(screen.getByRole("button", { name: "Preparing…" })).toBeDisabled();
    resolve(prepared); await screen.findByRole("alertdialog"); expect(screen.getAllByText(/Personal Finance/)).toHaveLength(2);
  });

  it("shows only bounded review evidence and requires explicit confirmation", async () => {
    await openReview(); expect(screen.getByText(/2 planned changes/)).toBeInTheDocument(); expect(screen.getByText(/aaaaaaaaaaaa…aaaa/)).toBeInTheDocument();
    expect(continueManagedWorkloadDeployment).not.toHaveBeenCalled();
    for (const hidden of ["opaque-operation", "fingerprint", "digest", "systemd", "docker", "secret", "command"]) expect(screen.queryByText(new RegExp(hidden, "i"))).not.toBeInTheDocument();
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
  });

  it("continues with only the opaque operation identity", async () => {
    continueManagedWorkloadDeployment.mockResolvedValue({ outcome: "dispatchStarted", operation: { operation: prepared.operation.operation, state: "dispatching" } });
    await openReview(); fireEvent.click(screen.getByRole("button", { name: "Confirm and start update" }));
    await waitFor(() => expect(continueManagedWorkloadDeployment).toHaveBeenCalledWith({ workloadId: "finance", operationId: "opaque-operation" }));
    await screen.findByText("Deployment in progress.");
  });

  it("returns PlanChanged to a fresh review requirement without a continue-anyway path", async () => {
    continueManagedWorkloadDeployment.mockResolvedValue({ outcome: "planChanged", operation: { operation: prepared.operation.operation, state: "planChanged" } });
    await openReview(); fireEvent.click(screen.getByRole("button", { name: "Confirm and start update" }));
    await screen.findByText(/plan changed before starting/i); expect(screen.queryByRole("button", { name: /continue anyway/i })).not.toBeInTheDocument(); expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("keeps an uncertain dispatch distinct and offers no retry", async () => {
    continueManagedWorkloadDeployment.mockResolvedValue({ outcome: "dispatchStarted", operation: { operation: prepared.operation.operation, state: "dispatchUncertain", problem: "dispatchOutcomeUncertain" } });
    await openReview(); fireEvent.click(screen.getByRole("button", { name: "Confirm and start update" }));
    await screen.findByText(/could not be confirmed/i); expect(screen.queryByRole("button", { name: /retry/i })).not.toBeInTheDocument();
  });

  it("reports definite pre-dispatch errors without claiming success or completion", async () => {
    continueManagedWorkloadDeployment.mockRejectedValue({ message: "The action was unavailable.", remediation: "Prepare again after fixing it." });
    await openReview(); fireEvent.click(screen.getByRole("button", { name: "Confirm and start update" }));
    await screen.findByText(/deployment did not start/i); expect(screen.queryByText(/deployment completed/i)).not.toBeInTheDocument();
  });

  it("renders persisted active state after reopen without preparing or continuing", async () => {
    listManagedWorkloadDeployments.mockResolvedValue([persisted("deploying")]);
    const { rerender } = render(<ManagedWorkloadDeployments deviceId="pi5"/>);
    await screen.findByText("Deployment in progress."); rerender(<ManagedWorkloadDeployments deviceId="pi5"/>);
    expect(prepareManagedWorkloadDeployment).not.toHaveBeenCalled(); expect(continueManagedWorkloadDeployment).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Check status" })).toBeInTheDocument(); expect(screen.queryByRole("button", { name: "Prepare update" })).toBeDisabled();
  });

  it("keeps uncertain dispatch safe and reconciles exactly once", async () => {
    listManagedWorkloadDeployments.mockResolvedValue([persisted("dispatchUncertain")]); let resolve!: (value: ReturnType<typeof persisted>["deployment"]) => void;
    reconcileManagedWorkloadDeployment.mockReturnValue(new Promise(value => { resolve = value; }));
    render(<ManagedWorkloadDeployments deviceId="pi5"/>); const check = await screen.findByRole("button", { name: "Check status" });
    expect(screen.getByText(/could not be confirmed/i)).toBeInTheDocument(); expect(screen.queryByRole("button", { name: /retry/i })).not.toBeInTheDocument();
    fireEvent.click(check); fireEvent.click(check); expect(reconcileManagedWorkloadDeployment).toHaveBeenCalledTimes(1); expect(reconcileManagedWorkloadDeployment).toHaveBeenCalledWith("finance");
    resolve({ operation, state: "deploying", observationDeadline: undefined }); await screen.findByText("Deployment in progress.");
  });

  it("renders still-running and verification states without locally claiming completion", async () => {
    listManagedWorkloadDeployments.mockResolvedValue([persisted("stillRunning")]); const { rerender } = render(<ManagedWorkloadDeployments deviceId="pi5"/>);
    await screen.findByText(/Automatic observation has ended/i); expect(screen.getByRole("button", { name: "Check status" })).toBeInTheDocument();
    listManagedWorkloadDeployments.mockResolvedValue([persisted("revisionVerified")]); rerender(<ManagedWorkloadDeployments deviceId="next"/>);
    await screen.findByText("Verifying deployment…"); expect(screen.queryByText("Deployment complete.")).not.toBeInTheDocument();
  });

  it("shows completion and terminal failures only when backend persists them", async () => {
    listManagedWorkloadDeployments.mockResolvedValue([persisted("completed")]); const { rerender } = render(<ManagedWorkloadDeployments deviceId="pi5"/>);
    await screen.findByText("Deployment complete.");
    listManagedWorkloadDeployments.mockResolvedValue([persisted("verificationFailed")]); rerender(<ManagedWorkloadDeployments deviceId="next"/>); await screen.findByText(/verification failed/i);
    listManagedWorkloadDeployments.mockResolvedValue([persisted("deploymentFailed")]); rerender(<ManagedWorkloadDeployments deviceId="again"/>); await screen.findByText(/^Deployment failed\./);
  });

  it("preserves the last known lifecycle after temporary reconciliation errors", async () => {
    listManagedWorkloadDeployments.mockResolvedValue([persisted("deploying")]); reconcileManagedWorkloadDeployment.mockRejectedValue({ message: "Connection timed out.", remediation: "Try later." });
    render(<ManagedWorkloadDeployments deviceId="pi5"/>); fireEvent.click(await screen.findByRole("button", { name: "Check status" }));
    await screen.findByText(/last known deployment state is still shown/i); expect(screen.getByText("Deployment in progress.")).toBeInTheDocument();
  });

  it("observes an active persisted operation every five seconds without overlapping requests", async () => {
    vi.useFakeTimers(); vi.setSystemTime(new Date("2026-08-14T16:00:00Z")); const deadline = new Date(Date.now() + 60_000).toISOString();
    listManagedWorkloadDeployments.mockResolvedValue([persisted("deploying", deadline)]); let resolve!: (value: ReturnType<typeof persisted>["deployment"]) => void;
    reconcileManagedWorkloadDeployment.mockReturnValue(new Promise(value => { resolve = value; })); render(<ManagedWorkloadDeployments deviceId="pi5"/>); await vi.advanceTimersByTimeAsync(0);
    await vi.advanceTimersByTimeAsync(5_000); expect(reconcileManagedWorkloadDeployment).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(10_000); expect(reconcileManagedWorkloadDeployment).toHaveBeenCalledTimes(1);
    await act(async () => { resolve({ operation, state: "deploying", observationDeadline: deadline }); }); await vi.advanceTimersByTimeAsync(5_000);
    expect(reconcileManagedWorkloadDeployment).toHaveBeenCalledTimes(2); expect(prepareManagedWorkloadDeployment).not.toHaveBeenCalled(); expect(continueManagedWorkloadDeployment).not.toHaveBeenCalled();
  });

  it("observes uncertain dispatch but stops automatically for returned terminal and still-running states", async () => {
    vi.useFakeTimers(); vi.setSystemTime(new Date("2026-08-14T16:00:00Z")); const deadline = new Date(Date.now() + 60_000).toISOString();
    listManagedWorkloadDeployments.mockResolvedValueOnce([persisted("dispatchUncertain", deadline)]).mockResolvedValue([persisted("completed", deadline)]);
    reconcileManagedWorkloadDeployment.mockResolvedValue({ operation, state: "completed", observationDeadline: deadline }); render(<ManagedWorkloadDeployments deviceId="pi5"/>); await vi.advanceTimersByTimeAsync(0);
    await vi.advanceTimersByTimeAsync(5_000); expect(reconcileManagedWorkloadDeployment).toHaveBeenCalledTimes(1); await vi.advanceTimersByTimeAsync(15_000); expect(reconcileManagedWorkloadDeployment).toHaveBeenCalledTimes(1);
    listManagedWorkloadDeployments.mockReset().mockResolvedValue([persisted("stillRunning", deadline)]); reconcileManagedWorkloadDeployment.mockReset(); render(<ManagedWorkloadDeployments deviceId="next"/>); await vi.advanceTimersByTimeAsync(10_000); expect(reconcileManagedWorkloadDeployment).not.toHaveBeenCalled();
  });

  it("uses the persisted deadline, resumes before it, and cancels observation on unmount", async () => {
    vi.useFakeTimers(); vi.setSystemTime(new Date("2026-08-14T16:00:00Z")); const deadline = new Date(Date.now() + 5_100).toISOString();
    listManagedWorkloadDeployments.mockResolvedValue([persisted("deploying", deadline)]); reconcileManagedWorkloadDeployment.mockResolvedValue({ operation, state: "deploying", observationDeadline: deadline });
    const { unmount } = render(<ManagedWorkloadDeployments deviceId="pi5"/>); await vi.advanceTimersByTimeAsync(0); await vi.advanceTimersByTimeAsync(5_000); expect(reconcileManagedWorkloadDeployment).toHaveBeenCalledTimes(1); unmount(); await vi.advanceTimersByTimeAsync(10_000); expect(reconcileManagedWorkloadDeployment).toHaveBeenCalledTimes(1);
    reconcileManagedWorkloadDeployment.mockReset(); render(<ManagedWorkloadDeployments deviceId="next"/>); await vi.advanceTimersByTimeAsync(10_000); expect(reconcileManagedWorkloadDeployment).not.toHaveBeenCalled();
  });

  it("preserves state after automatic transport failure and permits a later bounded observation", async () => {
    vi.useFakeTimers(); vi.setSystemTime(new Date("2026-08-14T16:00:00Z")); const deadline = new Date(Date.now() + 20_000).toISOString();
    listManagedWorkloadDeployments.mockResolvedValue([persisted("deploying", deadline)]); reconcileManagedWorkloadDeployment.mockRejectedValueOnce({ message: "Connection timed out." }).mockResolvedValue({ operation, state: "deploying", observationDeadline: deadline });
    render(<ManagedWorkloadDeployments deviceId="pi5"/>); await vi.advanceTimersByTimeAsync(0); await vi.advanceTimersByTimeAsync(5_000); expect(screen.getByText("Deployment in progress.")).toBeInTheDocument(); await vi.advanceTimersByTimeAsync(5_000); expect(reconcileManagedWorkloadDeployment).toHaveBeenCalledTimes(2);
  });
});
