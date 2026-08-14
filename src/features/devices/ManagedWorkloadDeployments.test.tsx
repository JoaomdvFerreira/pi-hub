import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { listManagedWorkloadDeployments, prepareManagedWorkloadDeployment, continueManagedWorkloadDeployment } = vi.hoisted(() => ({ listManagedWorkloadDeployments: vi.fn(), prepareManagedWorkloadDeployment: vi.fn(), continueManagedWorkloadDeployment: vi.fn() }));
vi.mock("@/lib/tauri/managedWorkloads", () => ({ listManagedWorkloadDeployments, prepareManagedWorkloadDeployment, continueManagedWorkloadDeployment }));
import { ManagedWorkloadDeployments } from "./ManagedWorkloadDeployments";

const card = { workloadId: "finance", name: "Personal Finance", enabled: true, eligibleToPrepare: true };
const prepared = { operation: { operation: { workloadId: "finance", operationId: "opaque-operation" }, state: "prepared" }, reviewTargetRevision: "a".repeat(40), changeCount: 2 };

describe("ManagedWorkloadDeployments", () => {
  beforeEach(() => { listManagedWorkloadDeployments.mockReset().mockResolvedValue([card]); prepareManagedWorkloadDeployment.mockReset().mockResolvedValue(prepared); continueManagedWorkloadDeployment.mockReset(); });
  afterEach(cleanup);
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
});
