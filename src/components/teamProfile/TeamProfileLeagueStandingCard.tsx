import { Card, CardBody, CardHeader } from "../ui";

import { StatBox } from "./TeamProfile.primitives";
import type { LeagueStanding, TeamProfileTranslate } from "./TeamProfile.types";

interface TeamProfileLeagueStandingCardProps {
  standings: LeagueStanding | null;
  t: TeamProfileTranslate;
}

export default function TeamProfileLeagueStandingCard({
  standings,
  t,
}: TeamProfileLeagueStandingCardProps) {
  if (!standings) {
    return null;
  }

  const decisiveGames = standings.won + standings.lost;
  const winRate =
    decisiveGames > 0
      ? `${Math.round((standings.won / decisiveGames) * 100)}%`
      : "0%";

  return (
    <Card>
      <CardHeader>{t("teamProfile.leagueStanding")}</CardHeader>
      <CardBody>
        <div className="grid grid-cols-3 gap-2 text-center">
          <StatBox label={t("common.played")} value={standings.played} />
          <StatBox label={t("common.won")} value={standings.won} />
          <StatBox label={t("common.lost")} value={standings.lost} />
          <StatBox label={t("teamProfile.winRate")} value={winRate} />
          <StatBox label={t("common.pts")} value={standings.points} highlight />
        </div>
      </CardBody>
    </Card>
  );
}
