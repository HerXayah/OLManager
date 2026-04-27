import { Building2, Users } from "lucide-react";

import { Card, CardBody, CardHeader } from "../ui";
import type { TeamData } from "../../store/gameStore";
import type { TeamProfileTranslate } from "./TeamProfile.types";
import { InfoRow } from "./TeamProfile.primitives";

interface TeamProfileClubDetailsCardProps {
  team: TeamData;
  rosterSize: number;
  t: TeamProfileTranslate;
}

function resolveErlLeagueName(team: TeamData): string | null {
  if (team.team_kind !== "Academy") return null;

  const erlLeagueId = (team.academy as { erl_assignment?: { erl_league_id?: string } } | null | undefined)
    ?.erl_assignment?.erl_league_id;

  if (!erlLeagueId) return null;

  const normalized = erlLeagueId.toLowerCase();
  if (normalized === "liga-espanola" || normalized === "les") return "LES";
  if (normalized === "lfl") return "LFL";
  if (normalized === "prime-league" || normalized === "primeleague" || normalized === "prm") {
    return "Prime League";
  }

  return null;
}

export default function TeamProfileClubDetailsCard({
  team,
  rosterSize,
  t,
}: TeamProfileClubDetailsCardProps) {
  const erlLeagueName = resolveErlLeagueName(team);

  return (
    <Card>
      <CardHeader>{t("teamProfile.clubInfo")}</CardHeader>
      <CardBody>
        <div className="flex flex-col gap-3">
          <InfoRow
            icon={<Building2 className="w-4 h-4" />}
            label={erlLeagueName ? t("teamProfile.erl") : t("teamProfile.hq")}
            value={erlLeagueName ?? team.city}
          />
          <InfoRow
            icon={<Users className="w-4 h-4" />}
            label={t("teamProfile.activeRoster")}
            value={String(rosterSize)}
          />
        </div>
      </CardBody>
    </Card>
  );
}
