import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { GameStateData } from "../../store/gameStore";
import { MatchSnapshot } from "./types";
import { Badge, ThemeToggle } from "../ui";
import { ChevronRight, Mic, MessageSquare } from "lucide-react";
import { buildPressConferenceQuestions } from "./pressConferenceContent";

interface PressConferenceProps {
  snapshot: MatchSnapshot;
  gameState: GameStateData;
  userSide: "Home" | "Away";
  onFinish: () => void;
  onGameUpdate?: (game: GameStateData) => void;
}

interface AnswerPayload {
  question_id: string;
  response_id: string;
  effect_id: string;
  response_tone: string;
  response_text: string;
  question_text: string;
  player_id?: string;
}

const RECENT_PRESS_QUESTIONS_KEY = "olmanager:match:pressConference:recentQuestionIds";
const RECENT_PRESS_QUESTIONS_LIMIT = 12;

function readRecentPressQuestionIds(): string[] {
  try {
    const raw = window.localStorage.getItem(RECENT_PRESS_QUESTIONS_KEY);
    if (!raw) return [];

    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((id): id is string => typeof id === "string") : [];
  } catch {
    return [];
  }
}

function persistRecentPressQuestionIds(questionIds: string[]): void {
  if (questionIds.length === 0) return;

  try {
    const existing = readRecentPressQuestionIds();
    const merged = [...existing, ...questionIds].filter(
      (id, index, ids) => ids.lastIndexOf(id) === index,
    );
    window.localStorage.setItem(
      RECENT_PRESS_QUESTIONS_KEY,
      JSON.stringify(merged.slice(-RECENT_PRESS_QUESTIONS_LIMIT)),
    );
  } catch {
    // Ignore storage failures; press conference submission must remain unaffected.
  }
}

export default function PressConference({
  snapshot,
  gameState,
  userSide,
  onFinish,
  onGameUpdate,
}: PressConferenceProps) {
  const { t } = useTranslation();
  const [questions] = useState(() =>
    buildPressConferenceQuestions({
      snapshot,
      userSide,
      gameState,
      t,
      recentQuestionIds: readRecentPressQuestionIds(),
    }),
  );
  const [currentIdx, setCurrentIdx] = useState(0);
  const [answers, setAnswers] = useState<Record<string, string>>({});
  const [submitting, setSubmitting] = useState(false);

  const currentQ = questions[currentIdx];
  const isLastQuestion = currentIdx === questions.length - 1;
  const hasAnswered = currentQ ? !!answers[currentQ.id] : false;

  useEffect(() => {
    persistRecentPressQuestionIds(questions.map((question) => question.id));
  }, [questions]);

  const handleAnswer = (responseId: string) => {
    if (!currentQ) return;
    setAnswers((prev) => ({ ...prev, [currentQ.id]: responseId }));
  };

  const submitToBackend = async () => {
    setSubmitting(true);
    try {
      const payloads: AnswerPayload[] = questions
        .map((q) => {
          const rid = answers[q.id];
          const resp = q.responses.find((r) => r.id === rid);
          return {
            question_id: q.id,
            response_id: rid || "",
            effect_id: resp?.effectId || "",
            response_tone: resp?.tone || "",
            response_text: resp?.text || "",
            question_text: q.question,
            player_id: q.playerId || "",
          };
        })
        .filter((p) => p.response_id);

      const userTeamName =
        userSide === "Home" ? snapshot.home_team.name : snapshot.away_team.name;
      const userTeamId =
        userSide === "Home" ? snapshot.home_team.id : snapshot.away_team.id;
      const resultStr = `${snapshot.home_team.name} ${snapshot.home_score} - ${snapshot.away_score} ${snapshot.away_team.name}`;
      const quotes = payloads
        .filter((p) => p.response_text)
        .map((p) => `"${p.response_text}"`);
      const firstQuoteRaw = payloads[0]?.response_text ?? "";

      const prerenderedHeadline =
        quotes.length === 0
          ? t("match.pressReport.headlinePostMatch", { team: userTeamName, result: resultStr })
          : Math.random() < 0.5
            ? t("match.pressReport.headlineManagerQuote", { team: userTeamName, quote: firstQuoteRaw })
            : t("match.pressReport.headlinePressConf", { team: userTeamName, quote: firstQuoteRaw });

      let prerenderedBody: string;
      if (quotes.length > 1) {
        const bulletList = quotes.map((q) => `• ${q}`).join("\n");
        prerenderedBody =
          t("match.pressReport.bodyIntro", { result: resultStr, team: userTeamName }) +
          "\n\n" +
          bulletList +
          "\n\n" +
          t("match.pressReport.bodyOutro");
      } else if (quotes.length === 1) {
        prerenderedBody =
          t("match.pressReport.bodySingle", { team: userTeamName, result: resultStr }) +
          "\n\n" +
          quotes[0];
      } else {
        prerenderedBody = t("match.pressReport.bodyNone", { team: userTeamName, result: resultStr });
      }

      const result = await invoke<{
        game: GameStateData;
        morale_delta: number;
      }>("submit_press_conference", {
        answers: payloads,
        homeTeam: snapshot.home_team.name,
        awayTeam: snapshot.away_team.name,
        homeScore: snapshot.home_score,
        awayScore: snapshot.away_score,
        userTeamName: userTeamName,
        userTeamId: userTeamId,
        prerenderedBody,
        prerenderedHeadline,
      });
      if (result.game && onGameUpdate) {
        onGameUpdate(result.game);
      }
    } catch (err) {
      console.error("Failed to submit press conference:", err);
    } finally {
      setSubmitting(false);
      onFinish();
    }
  };

  const handleNext = () => {
    if (isLastQuestion) {
      submitToBackend();
    } else {
      setCurrentIdx((prev) => prev + 1);
    }
  };

  const userTeamName =
    userSide === "Home" ? snapshot.home_team.name : snapshot.away_team.name;

  return (
    <div className="min-h-screen bg-gray-100 text-gray-900 dark:bg-navy-900 dark:text-white flex flex-col transition-colors duration-300">
      {/* Header */}
      <header className="bg-linear-to-r from-gray-200 via-white to-gray-200 dark:from-navy-800 dark:via-navy-900 dark:to-navy-800 border-b border-gray-200 dark:border-navy-700 px-4 py-6 transition-colors duration-300">
        <div className="max-w-3xl mx-auto text-center relative">
          <ThemeToggle className="absolute right-0 top-0" />
          <div className="inline-flex items-center gap-2 px-4 py-1.5 bg-gray-200 dark:bg-navy-700 rounded-full mb-3 transition-colors duration-300">
            <Mic className="w-4 h-4 text-accent-400" />
            <span className="font-heading font-bold text-xs uppercase tracking-widest text-gray-700 dark:text-gray-300">
              {t("match.pressConference")}
            </span>
          </div>
          <p className="text-sm text-gray-500 dark:text-gray-400">
            {t("match.pressSubtitle", { team: userTeamName })}
          </p>
          <div className="flex items-center justify-center gap-1 mt-3">
            {questions.map((_, i) => (
              <div
                key={i}
                className={`w-8 h-1 rounded-full transition-colors ${
                  i < currentIdx
                    ? "bg-primary-500"
                    : i === currentIdx
                      ? "bg-primary-400"
                      : "bg-gray-300 dark:bg-navy-700"
                }`}
              />
            ))}
          </div>
        </div>
      </header>

      {/* Main content */}
      <div className="flex-1 flex items-center justify-center p-6">
        {currentQ && (
          <div className="max-w-2xl w-full">
            {/* Journalist */}
            <div className="flex items-start gap-4 mb-8">
               <div className="w-12 h-12 rounded-full bg-gray-200 dark:bg-navy-700 flex items-center justify-center flex-shrink-0 transition-colors duration-300">
                 <MessageSquare className="w-5 h-5 text-gray-500 dark:text-gray-400" />
               </div>
               <div>
                 <div className="flex items-center gap-2 mb-1">
                   <span className="font-heading font-bold text-sm text-gray-800 dark:text-gray-200">
                     {currentQ.journalist}
                   </span>
                  <Badge variant="neutral" size="sm">
                    {currentQ.outlet}
                  </Badge>
                </div>
                 <p className="text-lg text-gray-700 dark:text-gray-300 leading-relaxed italic">
                   "{currentQ.question}"
                 </p>
              </div>
            </div>

            {/* Responses */}
            <div className="flex flex-col gap-3 ml-16">
              {currentQ.responses.map((r) => {
                const isSelected = answers[currentQ.id] === r.id;
                return (
                  <button
                    key={r.id}
                    onClick={() => handleAnswer(r.id)}
                    disabled={hasAnswered}
                     className={`p-4 rounded-xl text-left transition-all ${
                       isSelected
                         ? "bg-primary-500/20 ring-2 ring-primary-500/50"
                       : hasAnswered
                          ? "bg-gray-200/70 dark:bg-navy-800/50 opacity-40"
                          : "bg-white hover:bg-gray-100 border border-gray-200 dark:bg-navy-800 dark:hover:bg-navy-700 dark:border-navy-700"
                     }`}
                   >
                    <div className="flex items-center gap-2 mb-1">
                      <Badge
                        variant={isSelected ? "primary" : "neutral"}
                        size="sm"
                      >
                        {r.tone}
                      </Badge>
                    </div>
                     <p
                       className={`text-sm ${isSelected ? "text-gray-800 dark:text-gray-200" : "text-gray-500 dark:text-gray-400"}`}
                     >
                       "{r.text}"
                    </p>
                  </button>
                );
              })}
            </div>

            {/* Next button */}
            {hasAnswered && (
              <div className="flex justify-end mt-6 ml-16">
                <button
                  onClick={handleNext}
                  className="flex items-center gap-2 px-6 py-3 bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 rounded-xl font-heading font-bold uppercase tracking-wider text-sm text-white shadow-lg shadow-primary-500/20 transition-all"
                >
                  {submitting
                    ? t("match.submitting")
                    : isLastQuestion
                      ? t("match.leaveConference")
                      : t("match.nextQuestion")}
                  <ChevronRight className="w-4 h-4" />
                </button>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Skip button */}
      <footer className="bg-white dark:bg-navy-800 border-t border-gray-200 dark:border-navy-700 px-6 py-3 transition-colors duration-300">
        <div className="max-w-3xl mx-auto flex justify-end">
          <button
            onClick={onFinish}
            className="text-xs font-heading uppercase tracking-wider text-gray-600 hover:text-gray-800 dark:text-gray-500 dark:hover:text-gray-300 transition-colors"
          >
            {t("match.skipConference")}
          </button>
        </div>
      </footer>
    </div>
  );
}
