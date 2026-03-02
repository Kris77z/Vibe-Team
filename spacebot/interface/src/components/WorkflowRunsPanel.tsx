import { useMemo, useState } from "react";
import type { WorkflowRunState } from "@/hooks/useLiveContext";

function statusTone(status: string) {
	switch (status) {
		case "completed":
			return "border-emerald-500/25 bg-emerald-500/5 text-emerald-200";
		case "failed":
		case "cancelled":
			return "border-rose-500/25 bg-rose-500/5 text-rose-200";
		default:
			return "border-sky-500/25 bg-sky-500/5 text-sky-200";
	}
}

function statusDot(status: string) {
	switch (status) {
		case "completed":
			return "bg-emerald-400";
		case "failed":
		case "cancelled":
			return "bg-rose-400";
		default:
			return "bg-sky-400";
	}
}

function progressLabel(run: WorkflowRunState) {
	if (run.storyTotal <= 0) return null;
	return `${run.storyDone}/${run.storyTotal} stories`;
}

function hasDetails(run: WorkflowRunState) {
	return Boolean(
		run.changes ||
		run.tests ||
		run.reviewDecision ||
		run.branch ||
		run.prUrl ||
		run.needsHumanAcceptance ||
		run.openQuestions.length > 0,
	);
}

export function WorkflowRunsPanel({ runs }: { runs: WorkflowRunState[] }) {
	const [expandedRuns, setExpandedRuns] = useState<Record<string, boolean>>({});
	if (runs.length === 0) return null;

	const sortedRuns = useMemo(() => runs, [runs]);

	return (
		<div className="rounded-lg border border-sky-500/25 bg-sky-500/5 px-3 py-2">
			<div className="mb-2 flex items-center gap-1.5 text-tiny text-sky-200">
				<div className="h-1.5 w-1.5 rounded-full bg-sky-400" />
				<span>
					{runs.length} workflow run{runs.length !== 1 ? "s" : ""}
				</span>
			</div>
			<div className="flex flex-col gap-1.5">
				{sortedRuns.map((run) => {
					const expanded = expandedRuns[run.runId] ?? false;
					const expandable = hasDetails(run);

					return (
						<div
							key={run.runId}
							className={`rounded-md border px-2.5 py-2 text-tiny ${statusTone(run.status)}`}
						>
							<div className="flex min-w-0 items-center gap-2">
								<div className={`h-1.5 w-1.5 rounded-full ${statusDot(run.status)} ${run.isTerminal ? "" : "animate-pulse"}`} />
								<span className="font-medium">
									{run.runNumber ? `#${run.runNumber}` : run.workflowId}
								</span>
								<span className="min-w-0 flex-1 truncate text-ink-dull">
									{run.workflowId}
								</span>
								{expandable && (
									<button
										type="button"
										onClick={() => setExpandedRuns((prev) => ({ ...prev, [run.runId]: !expanded }))}
										className="shrink-0 rounded px-1.5 py-0.5 text-ink-faint transition-colors hover:bg-app-box/30 hover:text-ink"
									>
										{expanded ? "▾" : "▸"}
									</button>
								)}
								<span className="shrink-0 capitalize">{run.status}</span>
							</div>
							<div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 pl-3.5 text-ink-faint">
								{run.currentStep && <span>step: {run.currentStep}</span>}
								{run.currentAgent && <span>agent: {run.currentAgent}</span>}
								{progressLabel(run) && <span>{progressLabel(run)}</span>}
							</div>
							{run.blockingReason && (
								<div className="mt-1 pl-3.5 text-rose-200/90">
									{run.blockingReason}
								</div>
							)}
							{run.resultSummary && !expanded && (
								<div className="mt-1 pl-3.5 text-ink-dull">
									{run.resultSummary}
								</div>
							)}
							{expanded && (
								<div className="mt-2 space-y-2 border-t border-white/5 pt-2 pl-3.5">
									{run.changes && (
										<div>
											<div className="mb-0.5 text-[11px] uppercase tracking-wide text-ink-faint">Changes</div>
											<div className="text-ink-dull">{run.changes}</div>
										</div>
									)}
									{run.tests && (
										<div>
											<div className="mb-0.5 text-[11px] uppercase tracking-wide text-ink-faint">Tests</div>
											<div className="text-ink-dull">{run.tests}</div>
										</div>
									)}
									{run.reviewDecision && (
										<div>
											<div className="mb-0.5 text-[11px] uppercase tracking-wide text-ink-faint">Review</div>
											<div className="text-ink-dull">{run.reviewDecision}</div>
										</div>
									)}
									{(run.branch || run.prUrl) && (
										<div className="flex flex-wrap gap-x-4 gap-y-1">
											{run.branch && <div className="text-ink-dull">branch: {run.branch}</div>}
											{run.prUrl && (
												<a
													href={run.prUrl}
													target="_blank"
													rel="noreferrer"
													className="text-sky-200 underline underline-offset-2"
												>
													PR
												</a>
											)}
										</div>
									)}
									{run.needsHumanAcceptance && (
										<div className="text-amber-200">Human acceptance required</div>
									)}
									{run.openQuestions.length > 0 && (
										<div>
											<div className="mb-0.5 text-[11px] uppercase tracking-wide text-ink-faint">Open Questions</div>
											<div className="space-y-1">
												{run.openQuestions.map((question, index) => (
													<div key={`${run.runId}-question-${index}`} className="text-ink-dull">
														{question}
													</div>
												))}
											</div>
										</div>
									)}
								</div>
							)}
						</div>
					);
				})}
			</div>
		</div>
	);
}
