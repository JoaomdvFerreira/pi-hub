import { useState } from "react";
import { Loader2, Play, RotateCw, Square } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { performContainerAction } from "@/lib/tauri/containers";
import type { ApplicationError } from "@/types/settings";
import type { ContainerAction, DockerContainerState } from "@/types/snapshot";

function isApplicationError(err: unknown): err is ApplicationError {
  return typeof err === "object" && err !== null && "code" in err && "message" in err;
}

const ACTION_LABEL: Record<ContainerAction, string> = {
  start: "Start",
  stop: "Stop",
  restart: "Restart",
};

const ACTION_ICON: Record<ContainerAction, typeof Play> = {
  start: Play,
  stop: Square,
  restart: RotateCw,
};

interface ContainerActionsCellProps {
  deviceId: string;
  containerId: string;
  containerName: string;
  state: DockerContainerState;
}

export function ContainerActionsCell({
  deviceId,
  containerId,
  containerName,
  state,
}: ContainerActionsCellProps) {
  const [running, setRunning] = useState<ContainerAction | null>(null);
  const [error, setError] = useState<string | null>(null);

  const availableActions: ContainerAction[] =
    state === "running"
      ? ["restart", "stop"]
      : state === "restarting"
        ? []
        : ["start"];

  async function handleConfirm(action: ContainerAction) {
    setRunning(action);
    setError(null);
    try {
      await performContainerAction(deviceId, containerId, action);
    } catch (err) {
      setError(isApplicationError(err) ? err.message : `Could not ${action} this container.`);
    } finally {
      setRunning(null);
    }
  }

  if (availableActions.length === 0) {
    return <span className="text-xs text-muted-foreground">—</span>;
  }

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center gap-1.5">
        {availableActions.map((action) => {
          const Icon = ACTION_ICON[action];
          return (
            <AlertDialog key={action}>
              <AlertDialogTrigger asChild>
                <Button
                  variant="outline"
                  size="icon"
                  className="size-6.5"
                  title={`${ACTION_LABEL[action]} container`}
                  disabled={running !== null}
                >
                  {running === action ? (
                    <Loader2 className="size-3 animate-spin" />
                  ) : (
                    <Icon className="size-3" />
                  )}
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>
                    {ACTION_LABEL[action]} "{containerName}"?
                  </AlertDialogTitle>
                  <AlertDialogDescription>
                    This runs `docker {action}` for this container on the device over SSH. The
                    dashboard will reflect the result on the next refresh.
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel>Cancel</AlertDialogCancel>
                  <AlertDialogAction onClick={() => handleConfirm(action)}>
                    {ACTION_LABEL[action]}
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          );
        })}
      </div>
      {error ? <p className="text-[10.5px] text-destructive">{error}</p> : null}
    </div>
  );
}
