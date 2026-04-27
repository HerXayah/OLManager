import { useTranslation } from "react-i18next";
import type { GameStateData } from "../../store/gameStore";
import TacticsTab from "../tactics/TacticsTab";

interface MatchTacticsStageProps {
  gameState: GameStateData;
  onGameUpdate: (next: GameStateData) => void;
  onContinue: () => void;
  onRunParallelSims: () => void;
  isRunningParallelSims: boolean;
  parallelSimsFeedback: string | null;
}

export default function MatchTacticsStage({
  gameState,
  onGameUpdate,
  onContinue,
  onRunParallelSims,
  isRunningParallelSims,
  parallelSimsFeedback,
}: MatchTacticsStageProps) {
  const { t } = useTranslation();

  return (
    <div className="min-h-screen bg-gray-100 dark:bg-navy-900 transition-colors duration-300 p-4 md:p-6">
      <div className="max-w-7xl mx-auto">
        <div className="mb-4 rounded-xl border border-primary-200 dark:border-primary-900/50 bg-primary-50/70 dark:bg-primary-900/20 p-4 flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
          <div>
            <p className="text-xs font-heading uppercase tracking-widest text-primary-700 dark:text-primary-300">
              {t("match.tactics")}
            </p>
            <p className="text-sm text-primary-900/90 dark:text-primary-100/90">
              {t("match.tacticsBeforeLive")}
            </p>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={onRunParallelSims}
              className="px-4 py-2 rounded-lg bg-primary-700 hover:bg-primary-600 text-white font-heading uppercase tracking-wider text-xs"
            >
              {isRunningParallelSims
                ? t("match.stopParallelSims")
                : t("match.runParallelSims")}
            </button>

            <button
              onClick={onContinue}
              disabled={isRunningParallelSims}
              className="px-4 py-2 rounded-lg bg-primary-600 hover:bg-primary-500 disabled:bg-primary-900/60 disabled:cursor-not-allowed text-white font-heading uppercase tracking-wider text-xs"
            >
              {t("match.startLive")}
            </button>
          </div>
        </div>

        {parallelSimsFeedback ? (
          <p className="mb-4 text-xs text-primary-800 dark:text-primary-200">
            {parallelSimsFeedback}
          </p>
        ) : null}

        <TacticsTab
          gameState={gameState}
          onSelectPlayer={() => {}}
          onGameUpdate={onGameUpdate}
        />
      </div>
    </div>
  );
}
