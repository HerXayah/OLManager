import { useState, useMemo } from "react";
import { GameStateData, PlayerSelectionOptions } from "../../store/gameStore";
import { Card, CardBody, Badge, Select, CountryFlag } from "../ui";
import {
  Search,
  Filter,
  ArrowUpDown,
  ChevronLeft,
  ChevronRight,
  ChevronsLeft,
  ChevronsRight,
} from "lucide-react";
import {
  getTeamName,
  calcAge,
  formatVal,
} from "../../lib/helpers";
import { useTranslation } from "react-i18next";
import { calculateLolOvr } from "../../lib/lolPlayerStats";
import {
  getLolRoleForPlayer,
  LolRole,
} from "../squad/SquadTab.helpers";

interface PlayersListTabProps {
  gameState: GameStateData;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onSelectTeam: (id: string) => void;
}

type SortKey = "name" | "position" | "age" | "ovr" | "value" | "team";

function normalizeNick(value: string): string {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");
}

export default function PlayersListTab({
  gameState,
  onSelectPlayer,
  onSelectTeam,
}: PlayersListTabProps) {
  const { t } = useTranslation();
  const [search, setSearch] = useState("");
  const [posFilter, setPosFilter] = useState<LolRole | null>(null);
  const [teamFilter, setTeamFilter] = useState<string | null>(null);
  const [sortKey, setSortKey] = useState<SortKey>("ovr");
  const [sortAsc, setSortAsc] = useState(false);
  const [statusFilter, setStatusFilter] = useState<"all" | "transfer" | "loan">(
    "all",
  );
  const [page, setPage] = useState(1);
  const pageSize = 30;

  const handleSort = (key: SortKey) => {
    if (sortKey === key) setSortAsc(!sortAsc);
    else {
      setSortKey(key);
      setSortAsc(key === "name");
    }
  };

  // Reset page when filters change
  const filterKey = `${search}|${posFilter}|${teamFilter}|${statusFilter}|${sortKey}|${sortAsc}`;
  useMemo(() => setPage(1), [filterKey]);

  const dedupedPlayers = useMemo(() => {
    const byNick = new Map<string, (typeof gameState.players)[number]>();

    gameState.players.forEach((player) => {
      const nick = normalizeNick(player.match_name || "");
      if (!nick) return;

      const existing = byNick.get(nick);
      if (!existing) {
        byNick.set(nick, player);
        return;
      }

      const currentIsContracted = player.team_id !== null;
      const existingIsContracted = existing.team_id !== null;
      if (currentIsContracted && !existingIsContracted) {
        byNick.set(nick, player);
      }
    });

    return [...byNick.values()];
  }, [gameState.players]);

  let filtered = dedupedPlayers.filter((p) => {
    if (search.length >= 2) {
      const q = search.toLowerCase();
      if (
        !p.full_name.toLowerCase().includes(q) &&
        !p.match_name.toLowerCase().includes(q) &&
        !p.nationality.toLowerCase().includes(q)
      )
        return false;
    }
    if (posFilter && getLolRoleForPlayer(p) !== posFilter) return false;
    if (teamFilter && p.team_id !== teamFilter) return false;
    if (statusFilter === "transfer" && !p.transfer_listed) return false;
    if (statusFilter === "loan" && !p.loan_listed) return false;
    return true;
  });

  const posOrder: Record<LolRole, number> = {
    TOP: 1,
    JUNGLE: 2,
    MID: 3,
    ADC: 4,
    SUPPORT: 5,
  };

  filtered.sort((a, b) => {
    let cmp = 0;
    switch (sortKey) {
      case "name":
        cmp = a.match_name.localeCompare(b.match_name);
        break;
      case "position":
        cmp = posOrder[getLolRoleForPlayer(a)] - posOrder[getLolRoleForPlayer(b)];
        break;
      case "age":
        cmp = calcAge(a.date_of_birth) - calcAge(b.date_of_birth);
        break;
      case "ovr":
        cmp = calculateLolOvr(a) - calculateLolOvr(b);
        break;
      case "value":
        cmp = (a.market_value || 0) - (b.market_value || 0);
        break;
      case "team":
        cmp = getTeamName(gameState.teams, a.team_id).localeCompare(
          getTeamName(gameState.teams, b.team_id),
        );
        break;
    }
    return sortAsc ? cmp : -cmp;
  });

  const positions: LolRole[] = ["TOP", "JUNGLE", "MID", "ADC", "SUPPORT"];
  const roleBadgeVariant: Record<LolRole, "danger" | "success" | "accent" | "primary" | "neutral"> = {
    TOP: "danger",
    JUNGLE: "success",
    MID: "accent",
    ADC: "primary",
    SUPPORT: "neutral",
  };

  return (
    <div className="max-w-6xl mx-auto">
      {/* Filters */}
      <div className="flex flex-wrap gap-3 mb-4 items-center">
        <div className="relative flex-1 min-w-[200px] max-w-sm">
          <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 dark:text-gray-500" />
          <input
            type="text"
            placeholder={t("players.searchPlaceholder")}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full pl-9 pr-3 py-2 rounded-lg bg-white dark:bg-navy-800 border border-gray-200 dark:border-navy-600 text-sm text-gray-800 dark:text-gray-200 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-primary-500/50"
          />
        </div>

        <div className="flex gap-1.5">
          <button
            onClick={() => setPosFilter(null)}
            className={`px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${
              !posFilter
                ? "bg-primary-500 text-white shadow-sm"
                : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"
            }`}
          >
            {t("players.allPos")}
          </button>
          {positions.map((pos) => (
            <button
              key={pos}
              onClick={() => setPosFilter(posFilter === pos ? null : pos)}
              className={`px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${
                posFilter === pos
                  ? "bg-primary-500 text-white shadow-sm"
                  : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"
              }`}
            >
              {pos === "JUNGLE" ? "JG" : pos}
            </button>
          ))}
        </div>

        <div className="flex gap-1.5">
          <button
            onClick={() => setStatusFilter("all")}
            className={`px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${statusFilter === "all" ? "bg-primary-500 text-white shadow-sm" : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"}`}
          >
            {t("common.all")}
          </button>
          <button
            onClick={() => setStatusFilter("transfer")}
            className={`px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${statusFilter === "transfer" ? "bg-accent-500 text-white shadow-sm" : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"}`}
          >
            {t("transfers.transfer")}
          </button>
          <button
            onClick={() => setStatusFilter("loan")}
            className={`px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${statusFilter === "loan" ? "bg-blue-500 text-white shadow-sm" : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"}`}
          >
            {t("transfers.loan")}
          </button>
        </div>

        <Select
          value={teamFilter || ""}
          onChange={(e) => setTeamFilter(e.target.value || null)}
          selectSize="sm"
          className="min-w-44 font-heading font-bold uppercase tracking-wider"
        >
          <option value="">{t("players.allTeams")}</option>
          {gameState.teams.map((tm) => (
            <option key={tm.id} value={tm.id}>
              {tm.name}
            </option>
          ))}
        </Select>
      </div>

      <p className="text-xs text-gray-400 dark:text-gray-500 mb-3 font-heading uppercase tracking-wider">
        <Filter className="w-3.5 h-3.5 inline mr-1 -mt-0.5" />
        {t("players.nPlayersFound", { count: filtered.length })}
      </p>

      {/* Players table */}
      <Card>
        <CardBody className="p-0">
          <div className="overflow-x-auto">
            <table className="w-full text-left border-collapse">
              <thead>
                <tr className="bg-gray-50 dark:bg-navy-800 border-b border-gray-200 dark:border-navy-600 text-xs">
                  <SortHeader
                    label={t("common.position")}
                    sortKey="position"
                    current={sortKey}
                    asc={sortAsc}
                    onClick={handleSort}
                  />
                  <SortHeader
                    label={t("common.name")}
                    sortKey="name"
                    current={sortKey}
                    asc={sortAsc}
                    onClick={handleSort}
                  />
                  <SortHeader
                    label={t("common.age")}
                    sortKey="age"
                    current={sortKey}
                    asc={sortAsc}
                    onClick={handleSort}
                  />
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("common.nationality")}
                  </th>
                  <SortHeader
                    label={t("common.team")}
                    sortKey="team"
                    current={sortKey}
                    asc={sortAsc}
                    onClick={handleSort}
                  />
                  <SortHeader
                    label={t("common.value")}
                    sortKey="value"
                    current={sortKey}
                    asc={sortAsc}
                    onClick={handleSort}
                  />
                  <SortHeader
                    label={t("common.ovr")}
                    sortKey="ovr"
                    current={sortKey}
                    asc={sortAsc}
                    onClick={handleSort}
                  />
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("common.status")}
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
                {filtered
                  .slice((page - 1) * pageSize, page * pageSize)
                  .map((player) => {
                    const ovr = calculateLolOvr(player);
                    const age = calcAge(player.date_of_birth);
                    return (
                      <tr
                        key={player.id}
                        onClick={() => onSelectPlayer(player.id)}
                        className="hover:bg-gray-50 dark:hover:bg-navy-700/50 transition-colors cursor-pointer group"
                      >
                        <td className="py-2.5 px-4">
                          <Badge
                            variant={roleBadgeVariant[getLolRoleForPlayer(player)]}
                            size="sm"
                          >
                            {getLolRoleForPlayer(player) === "JUNGLE"
                              ? "JG"
                              : getLolRoleForPlayer(player)}
                          </Badge>
                        </td>
                        <td className="py-2.5 px-4">
                          <div className="min-w-0">
                            <p className="font-semibold text-sm text-gray-800 dark:text-gray-200 group-hover:text-primary-600 dark:group-hover:text-primary-400 transition-colors truncate">
                              {player.match_name}
                            </p>
                            <p className="text-[11px] text-gray-500 dark:text-gray-400 truncate">
                              {player.full_name}
                            </p>
                          </div>
                        </td>
                        <td className="py-2.5 px-4 text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                          {age}
                        </td>
                        <td
                          className="py-2.5 px-4 text-sm text-gray-500 dark:text-gray-400"
                          title={player.nationality}
                        >
                          <CountryFlag
                            code={player.nationality}
                            className="text-lg leading-none"
                          />
                        </td>
                        <td className="py-2.5 px-4">
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              if (player.team_id) onSelectTeam(player.team_id);
                            }}
                            disabled={!player.team_id}
                            className="text-sm text-gray-600 dark:text-gray-400 hover:text-primary-500 hover:underline transition-colors"
                          >
                            {getTeamName(gameState.teams, player.team_id)}
                          </button>
                        </td>
                        <td className="py-2.5 px-4 text-sm text-gray-600 dark:text-gray-400 font-medium">
                          {formatVal(player.market_value)}
                        </td>
                        <td className="py-2.5 px-4">
                          <span
                            className={`font-heading font-bold text-base tabular-nums ${
                              ovr >= 75
                                ? "text-primary-500"
                                : ovr >= 55
                                  ? "text-accent-500"
                                  : "text-gray-400"
                            }`}
                          >
                            {ovr}
                          </span>
                        </td>
                        <td className="py-2.5 px-4">
                          {player.transfer_listed && (
                            <Badge variant="accent" size="sm">
                              {t("transfers.transfer")}
                            </Badge>
                          )}
                          {player.loan_listed && (
                            <Badge variant="primary" size="sm">
                              {t("transfers.loan")}
                            </Badge>
                          )}
                          {player.injury && (
                            <Badge variant="danger" size="sm">
                              {t("common.injured")}
                            </Badge>
                          )}
                        </td>
                      </tr>
                    );
                  })}
              </tbody>
            </table>
            {filtered.length === 0 && (
              <div className="p-8 text-center text-gray-500 dark:text-gray-400 text-sm">
                {t("players.noMatch")}
              </div>
            )}
          </div>
          {/* Pagination */}
          {filtered.length > pageSize &&
            (() => {
              const totalPages = Math.ceil(filtered.length / pageSize);
              return (
                <div className="flex items-center justify-between px-4 py-3 border-t border-gray-100 dark:border-navy-600">
                  <p className="text-xs text-gray-400 dark:text-gray-500 font-heading">
                    {t("players.showingRange", {
                      from: (page - 1) * pageSize + 1,
                      to: Math.min(page * pageSize, filtered.length),
                      total: filtered.length,
                      defaultValue: `${(page - 1) * pageSize + 1}–${Math.min(page * pageSize, filtered.length)} of ${filtered.length}`,
                    })}
                  </p>
                  <div className="flex items-center gap-1">
                    <button
                      onClick={() => setPage(1)}
                      disabled={page === 1}
                      className="p-1.5 rounded-lg text-gray-400 hover:text-gray-700 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-navy-700 disabled:opacity-30 disabled:pointer-events-none transition-colors"
                    >
                      <ChevronsLeft className="w-4 h-4" />
                    </button>
                    <button
                      onClick={() => setPage((p) => Math.max(1, p - 1))}
                      disabled={page === 1}
                      className="p-1.5 rounded-lg text-gray-400 hover:text-gray-700 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-navy-700 disabled:opacity-30 disabled:pointer-events-none transition-colors"
                    >
                      <ChevronLeft className="w-4 h-4" />
                    </button>
                    <span className="px-3 py-1 text-xs font-heading font-bold text-gray-600 dark:text-gray-300">
                      {page} / {totalPages}
                    </span>
                    <button
                      onClick={() =>
                        setPage((p) => Math.min(totalPages, p + 1))
                      }
                      disabled={page === totalPages}
                      className="p-1.5 rounded-lg text-gray-400 hover:text-gray-700 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-navy-700 disabled:opacity-30 disabled:pointer-events-none transition-colors"
                    >
                      <ChevronRight className="w-4 h-4" />
                    </button>
                    <button
                      onClick={() => setPage(totalPages)}
                      disabled={page === totalPages}
                      className="p-1.5 rounded-lg text-gray-400 hover:text-gray-700 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-navy-700 disabled:opacity-30 disabled:pointer-events-none transition-colors"
                    >
                      <ChevronsRight className="w-4 h-4" />
                    </button>
                  </div>
                </div>
              );
            })()}
        </CardBody>
      </Card>
    </div>
  );
}

function SortHeader({
  label,
  sortKey,
  current,
  onClick,
}: {
  label: string;
  sortKey: SortKey;
  current: SortKey;
  asc: boolean;
  onClick: (k: SortKey) => void;
}) {
  const isActive = current === sortKey;
  return (
    <th
      onClick={() => onClick(sortKey)}
      className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 cursor-pointer hover:text-gray-700 dark:hover:text-gray-200 transition-colors select-none"
    >
      <span className="flex items-center gap-1">
        {label}
        <ArrowUpDown
          className={`w-3 h-3 ${isActive ? "text-primary-500" : "text-gray-300 dark:text-navy-600"}`}
        />
      </span>
    </th>
  );
}
