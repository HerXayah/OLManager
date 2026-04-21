import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import TrainingSettingsPanel from "./TrainingSettingsPanel";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "training.weeklySchedule") return "Weekly Schedule";
      if (key === "training.trainingFocus") return "Training Focus";
      if (key === "training.intensity") return "Intensity";
      if (key === "training.trainingAppliedNote") return "Applied note";
      if (key === "training.recoveryNote") return "Recovery note";
      if (key === "training.currentlyTraining") {
        return `Training ${params?.attrs} at ${params?.intensity}`;
      }
      if (key === "training.todayIs") return `${params?.day} is ${params?.type}`;
      if (key === "training.aTrainingDay") return "a training day";
      if (key === "training.aRestDay") return "a rest day";
      if (key.startsWith("training.schedules.")) return key.replace("training.schedules.", "");
      if (key.startsWith("training.focuses.")) return key.replace("training.focuses.", "");
      if (key.startsWith("training.intensities.")) return key.replace("training.intensities.", "");
      if (key.startsWith("training.days.")) return key.replace("training.days.", "");
      if (key.startsWith("playerProfile.lolStats.")) return key.replace("playerProfile.lolStats.", "");
      return key;
    },
  }),
}));

describe("TrainingSettingsPanel", () => {
  it("renders the current training schedule, focus, and applied note", () => {
    render(
      <TrainingSettingsPanel
        currentFocus="Scrims"
        currentIntensity="Medium"
        currentSchedule="Balanced"
        isSaving={false}
        todayWeekday={1}
        isTodayTraining={true}
        activeFocusAttrs={["macro", "teamfighting"]}
        onSetTraining={vi.fn()}
        onSetSchedule={vi.fn()}
        scheduleIds={["Intense", "Balanced", "Light"]}
        scheduleIcons={{ Intense: "I", Balanced: "B", Light: "L" }}
        scheduleColors={{ Intense: "text-red", Balanced: "text-blue", Light: "text-sky" }}
        dayKeys={["mon", "tue", "wed", "thu", "fri", "sat", "sun"]}
        trainingFocusIds={["Scrims", "VODReview", "MentalResetRecovery"]}
        trainingFocusIcons={{ Scrims: "S", VODReview: "V", MentalResetRecovery: "R" }}
        trainingFocusAttrs={{ Scrims: ["macro", "teamfighting"], VODReview: ["consistency"], MentalResetRecovery: [] }}
        intensityIds={["Low", "Medium", "High"]}
        intensityColors={{ Low: "text-blue", Medium: "text-yellow", High: "text-red" }}
      />,
    );

    expect(screen.getByText("Weekly Schedule")).toBeInTheDocument();
    expect(screen.getByText("Training Focus")).toBeInTheDocument();
    expect(screen.getByText(/Training macro, teamfighting at Medium.label/)).toBeInTheDocument();
    expect(screen.getByText(/tue is a training day/)).toBeInTheDocument();
  });

  it("wires schedule, focus, and intensity actions through callbacks", () => {
    const onSetTraining = vi.fn();
    const onSetSchedule = vi.fn();

    render(
      <TrainingSettingsPanel
        currentFocus="Scrims"
        currentIntensity="Medium"
        currentSchedule="Balanced"
        isSaving={false}
        todayWeekday={1}
        isTodayTraining={true}
        activeFocusAttrs={["macro"]}
        onSetTraining={onSetTraining}
        onSetSchedule={onSetSchedule}
        scheduleIds={["Intense", "Balanced", "Light"]}
        scheduleIcons={{ Intense: "I", Balanced: "B", Light: "L" }}
        scheduleColors={{ Intense: "text-red", Balanced: "text-blue", Light: "text-sky" }}
        dayKeys={["mon", "tue", "wed", "thu", "fri", "sat", "sun"]}
        trainingFocusIds={["Scrims", "VODReview", "MentalResetRecovery"]}
        trainingFocusIcons={{ Scrims: "S", VODReview: "V", MentalResetRecovery: "R" }}
        trainingFocusAttrs={{ Scrims: ["macro"], VODReview: ["consistency"], MentalResetRecovery: [] }}
        intensityIds={["Low", "Medium", "High"]}
        intensityColors={{ Low: "text-blue", Medium: "text-yellow", High: "text-red" }}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /Intense.label/i }));
    fireEvent.click(screen.getByRole("button", { name: /VODReview.label/i }));
    fireEvent.click(screen.getByRole("button", { name: /High.label/i }));

    expect(onSetSchedule).toHaveBeenCalledWith("Intense");
    expect(onSetTraining).toHaveBeenCalledWith("VODReview", "Medium");
    expect(onSetTraining).toHaveBeenCalledWith("Scrims", "High");
  });
});
