import { invoke } from "@tauri-apps/api/core";
import type { ManagedWorkloadCard, ManagedWorkloadContinuation, ManagedWorkloadOperationRef, PreparedManagedWorkload } from "@/types/managedWorkloads";

export function listManagedWorkloadDeployments(deviceId: string): Promise<ManagedWorkloadCard[]> { return invoke("list_managed_workload_deployments", { deviceId }); }
export function prepareManagedWorkloadDeployment(workloadId: string): Promise<PreparedManagedWorkload> { return invoke("prepare_managed_workload_deployment", { request: { workloadId } }); }
export function continueManagedWorkloadDeployment(operation: ManagedWorkloadOperationRef): Promise<ManagedWorkloadContinuation> { return invoke("continue_managed_workload_deployment", { operation }); }
