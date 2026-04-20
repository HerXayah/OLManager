import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { sendScout } from "./scoutingService";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("scoutingService", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  it("calls the send scout backend command", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(sendScout("staff-1", "player-1")).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("send_scout", {
      scoutId: "staff-1",
      playerId: "player-1",
    });
  });
});