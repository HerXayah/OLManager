import type { ScoutingAssignment, StaffData } from "../../store/gameStore";

export function scoutMaxSlots(ability: number): number {
  return ability >= 80
    ? 5
    : ability >= 60
      ? 4
      : ability >= 40
        ? 3
        : ability >= 20
          ? 2
          : 1;
}

export function scoutAssignmentCount(
  assignments: ScoutingAssignment[],
  scoutId: string,
): number {
  return assignments.filter((assignment) => assignment.scout_id === scoutId).length;
}

export function calculateAvailableScouts(
  scouts: StaffData[],
  assignments: ScoutingAssignment[],
): StaffData[] {
  return scouts.filter(
    (scout) =>
      scoutAssignmentCount(assignments, scout.id) <
      scoutMaxSlots(scout.attributes.judging_ability),
  );
}