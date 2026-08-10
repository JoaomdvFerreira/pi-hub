import { invoke } from "@tauri-apps/api/core";
import type { AdministrationOperation, AdministrationOperationType, ExpectedDisruption } from "@/types/administration";
export function performAdministrationOperation(deviceId: string, operationType: AdministrationOperationType): Promise<AdministrationOperation> { return invoke("perform_administration_operation", { deviceId, operationType }); }
export function getExpectedDisruption(deviceId: string): Promise<ExpectedDisruption | null> { return invoke("get_expected_disruption", { deviceId }); }
