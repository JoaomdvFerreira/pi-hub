import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DeviceAdministration } from "./DeviceAdministration";
import { ADMINISTRATION_ACTIONS } from "./DeviceAdministration.actions";

describe("M10 device administration", () => {
  it("exposes exactly the approved operations and no power-on control", () => {
    expect(ADMINISTRATION_ACTIONS.map((action) => action.label)).toEqual(["Restart Docker", "Restart Tailscale", "Restart Device", "Shut Down Device"]);
    expect(ADMINISTRATION_ACTIONS.every((action) => !action.label.includes("Power On"))).toBe(true);
  });

  it("explains the shutdown power-on boundary before confirmation", () => {
    render(<DeviceAdministration deviceId="pi5" deviceName="Pi 5" />);
    expect(screen.getByText(/does not provide Power On/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Shut Down Device" })).toBeInTheDocument();
  });
});
