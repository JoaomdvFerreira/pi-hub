import { Power, RotateCw, Server, Wifi } from "lucide-react";
import type { AdministrationOperationType } from "@/types/administration";

export const ADMINISTRATION_ACTIONS: {
  type: AdministrationOperationType;
  label: string;
  confirmation: string;
  danger?: boolean;
  Icon: typeof RotateCw;
}[] = [
  {
    type: "restartDocker",
    label: "Restart Docker",
    confirmation:
      "Running containers and hosted services may stop or restart temporarily. Pi-Hub will verify Docker availability afterwards.",
    Icon: Server,
  },
  {
    type: "restartTailscale",
    label: "Restart Tailscale",
    confirmation:
      "Tailscale connectivity may temporarily drop. If this device is reached through Tailscale, SSH may disconnect while Pi-Hub attempts bounded recovery verification.",
    Icon: Wifi,
  },
  {
    type: "restartDevice",
    label: "Restart Device",
    confirmation:
      "The device will restart. Containers and services will temporarily stop, terminal and SSH sessions may disconnect, and Pi-Hub will wait for it to return online.",
    danger: true,
    Icon: RotateCw,
  },
  {
    type: "shutdownDevice",
    label: "Shut Down Device",
    confirmation:
      "This will stop all services and containers and disconnect active SSH terminal sessions. Pi-Hub cannot turn this device back on; power-on must be manual or provided by infrastructure outside Pi-Hub.",
    danger: true,
    Icon: Power,
  },
];
