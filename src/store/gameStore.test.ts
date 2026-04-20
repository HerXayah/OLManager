import { describe, it, expect, beforeEach } from "vitest";
import { useGameStore } from "./gameStore";
import type { GameStateData } from "./types";

// ---------------------------------------------------------------------------
// Reset store between tests
// ---------------------------------------------------------------------------

beforeEach(() => {
  useGameStore.setState({
    hasActiveGame: false,
    managerName: null,
    gameState: null,
    isDirty: false,
  });
});

// Minimal GameStateData stub — only the fields the store cares about
const makeGameState = (overrides: Partial<GameStateData> = {}): GameStateData => ({
  clock: { current_date: "2026-08-01", start_date: "2026-08-01" },
  manager: {
    id: "mgr1", first_name: "Test", last_name: "Manager",
    date_of_birth: "1985-01-01", nationality: "GB", team_id: "team1",
    satisfaction: 50, fan_approval: 50, reputation: 500,
    career_stats: { matches_managed: 0, wins: 0, draws: 0, losses: 0, trophies: 0, best_finish: null },
    career_history: [],
  },
  teams: [],
  players: [],
  staff: [],
  messages: [],
  league: null,
  news: [],
  board_objectives: [],
  scouting_assignments: [],
  ...overrides,
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("useGameStore", () => {
  it("starts with default values", () => {
    const state = useGameStore.getState();
    expect(state.hasActiveGame).toBe(false);
    expect(state.managerName).toBeNull();
    expect(state.gameState).toBeNull();
    expect(state.isDirty).toBe(false);
  });

  describe("setGameActive", () => {
    it("sets hasActiveGame and managerName", () => {
      useGameStore.getState().setGameActive(true, "John Doe");
      const state = useGameStore.getState();
      expect(state.hasActiveGame).toBe(true);
      expect(state.managerName).toBe("John Doe");
    });

    it("sets managerName to null when not provided", () => {
      useGameStore.getState().setGameActive(true);
      expect(useGameStore.getState().managerName).toBeNull();
    });

    it("can deactivate the game", () => {
      useGameStore.getState().setGameActive(true, "Test");
      useGameStore.getState().setGameActive(false);
      const state = useGameStore.getState();
      expect(state.hasActiveGame).toBe(false);
      expect(state.managerName).toBeNull();
    });
  });

  describe("setGameState", () => {
    it("stores game state data", () => {
      const gs = makeGameState();
      useGameStore.getState().setGameState(gs);
      expect(useGameStore.getState().gameState).toBe(gs);
    });

    it("marks state as dirty", () => {
      useGameStore.getState().setGameState(makeGameState());
      expect(useGameStore.getState().isDirty).toBe(true);
    });

    it("replaces previous game state", () => {
      const gs1 = makeGameState({ clock: { current_date: "2026-08-01", start_date: "2026-08-01" } });
      const gs2 = makeGameState({ clock: { current_date: "2026-09-01", start_date: "2026-08-01" } });
      useGameStore.getState().setGameState(gs1);
      useGameStore.getState().setGameState(gs2);
      expect(useGameStore.getState().gameState?.clock.current_date).toBe("2026-09-01");
    });
  });

  describe("clearGame", () => {
    it("resets all fields to initial state", () => {
      useGameStore.getState().setGameActive(true, "Test Manager");
      useGameStore.getState().setGameState(makeGameState());
      useGameStore.getState().clearGame();

      const state = useGameStore.getState();
      expect(state.hasActiveGame).toBe(false);
      expect(state.managerName).toBeNull();
      expect(state.gameState).toBeNull();
      expect(state.isDirty).toBe(false);
    });
  });

  describe("isDirty / markClean", () => {
    it("is false initially", () => {
      expect(useGameStore.getState().isDirty).toBe(false);
    });

    it("becomes true after setGameState", () => {
      useGameStore.getState().setGameState(makeGameState());
      expect(useGameStore.getState().isDirty).toBe(true);
    });

    it("resets to false after markClean", () => {
      useGameStore.getState().setGameState(makeGameState());
      useGameStore.getState().markClean();
      expect(useGameStore.getState().isDirty).toBe(false);
    });

    it("resets to false after clearGame", () => {
      useGameStore.getState().setGameState(makeGameState());
      useGameStore.getState().clearGame();
      expect(useGameStore.getState().isDirty).toBe(false);
    });
  });
});
