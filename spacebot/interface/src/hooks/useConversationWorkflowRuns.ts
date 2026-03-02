import { useEffect, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { api, type ConversationWorkflowRun } from "@/api/client";
import { useLiveContext, type WorkflowRunState } from "@/hooks/useLiveContext";

function mapRun(run: ConversationWorkflowRun): WorkflowRunState {
	return {
		runId: run.run_id,
		workflowId: run.workflow_id,
		status: run.status,
		runNumber: null,
		currentStep: run.current_step ?? null,
		currentAgent: run.current_agent ?? null,
		storyDone: run.story_done,
		storyTotal: run.story_total,
		blockingReason: run.blocking_reason ?? null,
		resultSummary: run.result_summary ?? null,
		changes: run.changes ?? null,
		tests: run.tests ?? null,
		reviewDecision: run.review_decision ?? null,
		branch: run.branch ?? null,
		prUrl: run.pr_url ?? null,
		needsHumanAcceptance: run.needs_human_acceptance ?? false,
		openQuestions: run.open_questions ?? [],
		isTerminal: run.is_terminal,
	};
}

export function useConversationWorkflowRuns(conversationId: string | null | undefined) {
	const { workflowRunsByConversation, hydrateWorkflowRuns } = useLiveContext();

	const query = useQuery({
		queryKey: ["antfarm-conversation-runs", conversationId],
		queryFn: () => api.antfarmConversationRuns(conversationId!),
		enabled: Boolean(conversationId),
		staleTime: 5_000,
	});

	useEffect(() => {
		if (!conversationId || !query.data) return;
		hydrateWorkflowRuns(
			conversationId,
			query.data.runs.map(mapRun),
		);
	}, [conversationId, query.data, hydrateWorkflowRuns]);

	const runs = useMemo(
		() => (conversationId ? workflowRunsByConversation[conversationId] ?? [] : []),
		[conversationId, workflowRunsByConversation],
	);

	return {
		runs,
		isLoading: query.isLoading,
	};
}
