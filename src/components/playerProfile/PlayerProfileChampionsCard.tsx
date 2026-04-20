import { Crown } from "lucide-react";
import { Card, CardBody, CardHeader } from "../ui";

interface ChampionMasteryItem {
  championId: string;
  championName: string;
  mastery: number;
  rank: "insignia" | 1 | 2 | 3;
  wr: number;
  games: number;
}

interface PlayerProfileChampionsCardProps {
  champions: ChampionMasteryItem[];
}

function championPortraitUrl(championId: string): string {
  return `https://ddragon.leagueoflegends.com/cdn/img/champion/tiles/${championId}_0.jpg`;
}

export default function PlayerProfileChampionsCard({ champions }: PlayerProfileChampionsCardProps) {
  return (
    <Card className="lg:col-span-2 min-h-[304px]">
      <CardHeader>Pool de campeones</CardHeader>
      <CardBody className="py-4 px-5">
        <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-2.5">
          {champions.map((item) => (
            <div
              key={`${item.rank}-${item.championId}`}
              className="relative rounded-xl overflow-hidden border border-[#22345d] min-h-[192px] bg-[#111f3d]"
            >
              <div
                className="absolute inset-0 bg-cover bg-center"
                style={{ backgroundImage: `url(${championPortraitUrl(item.championId)})` }}
              />
              <div className="absolute inset-0 bg-linear-to-b from-black/45 via-black/45 to-black/75" />

              <div className="relative z-10 p-2.5 h-full flex flex-col">
                <div className="flex items-start justify-between">
                  {item.rank === "insignia" ? (
                    <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[10px] font-heading font-bold uppercase tracking-wide bg-amber-500/20 text-amber-300 border border-amber-300/35">
                      <Crown className="w-3 h-3" /> Insignia
                    </span>
                  ) : (
                    <span className="inline-flex items-center px-2 py-0.5 rounded-md text-[10px] font-heading font-bold uppercase tracking-wide bg-white/20 text-white border border-white/35">
                      #{item.rank}
                    </span>
                  )}

                  <span className={`text-lg font-heading font-black ${item.wr >= 55 ? "text-emerald-300" : item.wr >= 48 ? "text-amber-300" : "text-rose-300"}`}>
                    {item.wr.toFixed(1)}% WR
                  </span>
                </div>

                <div className="mt-auto">
                  <p className="text-2xl font-heading font-black text-white leading-none truncate">{item.championName}</p>
                  <div className="mt-1 flex items-center justify-between text-white/90">
                    <p className="text-xs">Maestría {item.mastery}</p>
                    <p className="text-2xl font-heading font-black leading-none">{item.games} <span className="text-lg">Games</span></p>
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>
      </CardBody>
    </Card>
  );
}
