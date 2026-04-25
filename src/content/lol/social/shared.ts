export type UserSide = "Home" | "Away";
export type RegistrySide = "blue" | "red";

export const DEFAULT_LEAGUE_ID = "default";

export function registrySide(side: UserSide): RegistrySide {
  return side === "Home" ? "blue" : "red";
}
