import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DeviceVisibility } from "./DeviceVisibility";

describe("DeviceVisibility", () => {
  it("renders populated network, storage, and system summaries", () => {
    render(<DeviceVisibility network={{ interfaces: [{ name: "eth0", linkState: "up", ipv4Addresses: ["192.168.1.10/24"], ipv6Addresses: [] }], defaultRoute: { interface: "eth0", gateway: "192.168.1.1" }, dnsServers: ["1.1.1.1"] }} storage={{ filesystems: [{ source: "/dev/sda1", mountPoint: "/", filesystemType: "ext4", totalBytes: 1000, usedBytes: 400, usagePercent: 40, readOnly: false }] }} system={{ hostname: "pi5", operatingSystem: "Debian", logicalCoreCount: 4 }} />);
    expect(screen.getByText("192.168.1.10/24")).toBeInTheDocument();
    expect(screen.getByText(/root/)).toBeInTheDocument();
    expect(screen.getByText("Debian")).toBeInTheDocument();
  });
  it("makes unavailable data explicit", () => { render(<DeviceVisibility />); expect(screen.getAllByText("Unavailable for this refresh.")).toHaveLength(3); });
});
