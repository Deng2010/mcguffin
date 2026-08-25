import { useState } from "react";
import { createProblem } from "../../../services/problem.service";
import type { ContestOption, Difficulty } from "../../../types";
import type { ContestMode } from "../components/SubmitProblemForm";

interface UseSubmitFormOptions {
  contests: ContestOption[];
  onSubmitted: () => void;
}

export function useSubmitForm({ contests, onSubmitted }: UseSubmitFormOptions) {
  const [showSubmit, setShowSubmit] = useState(false);
  const [formTitle, setFormTitle] = useState("");
  const [contestMode, setContestMode] = useState<ContestMode>("none");
  const [selectedContestId, setSelectedContestId] = useState("");
  const [customContest, setCustomContest] = useState("");
  const [formDifficulty, setFormDifficulty] = useState<string>("Medium");
  const [formContent, setFormContent] = useState("");
  const [formSolution, setFormSolution] = useState("");
  const [formRemark, setFormRemark] = useState("");
  const [submitted, setSubmitted] = useState(false);
  const [formError, setFormError] = useState("");

  const getContestName = (): string => {
    if (contestMode === "select" && selectedContestId) {
      const found = contests.find((c) => c.id === selectedContestId);
      return found?.name || "";
    }
    if (contestMode === "custom") return customContest;
    return "";
  };

  const getContestId = (): string | undefined => {
    if (contestMode === "select" && selectedContestId) return selectedContestId;
    return undefined;
  };

  const resetForm = () => {
    setFormTitle("");
    setFormContent("");
    setFormSolution("");
    setFormRemark("");
    setContestMode("none");
    setSelectedContestId("");
    setCustomContest("");
    setFormDifficulty("Medium");
  };

  const handleSubmitProblem = async (e: React.FormEvent) => {
    e.preventDefault();
    const contest = getContestName();
    const contest_id = getContestId();
    try {
      await createProblem({
        title: formTitle,
        contest,
        contest_id,
        difficulty: formDifficulty as Difficulty,
        content: formContent,
        solution: formSolution.trim() ? formSolution : undefined,
        remark: formRemark.trim() ? formRemark : undefined,
      });
      setSubmitted(true);
      setFormError("");
      setTimeout(() => {
        setSubmitted(false);
        setShowSubmit(false);
        resetForm();
        onSubmitted();
      }, 2000);
    } catch (err) {
      setFormError(`${err}`);
    }
  };

  return {
    showSubmit,
    setShowSubmit,
    formTitle,
    setFormTitle,
    contestMode,
    setContestMode,
    selectedContestId,
    setSelectedContestId,
    customContest,
    setCustomContest,
    formDifficulty,
    setFormDifficulty,
    formContent,
    setFormContent,
    formSolution,
    setFormSolution,
    formRemark,
    setFormRemark,
    submitted,
    formError,
    handleSubmitProblem,
  };
}
