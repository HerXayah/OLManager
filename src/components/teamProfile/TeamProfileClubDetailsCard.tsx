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

export default function TeamProfileClubDetailsCard({
  team,
  rosterSize,
  t,
}: TeamProfileClubDetailsCardProps) {
  return (
    <Card>
      <CardHeader>{t("teamProfile.clubInfo")}</CardHeader>
      <CardBody>
        <div className="flex flex-col gap-3">
          <InfoRow
            icon={<Building2 className="w-4 h-4" />}
            label={t("teamProfile.hq")}
            value={team.city}
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
