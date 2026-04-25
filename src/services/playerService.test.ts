import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { startPotentialResearch } from "./playerService";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("playerService", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  it("calls start potential research backend command", async () => {
    const response = { manager: { id: "mgr-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(startPotentialResearch("player-1")).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("start_potential_research", {
      playerId: "player-1",
    });
  });
});
