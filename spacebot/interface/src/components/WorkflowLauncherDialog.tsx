import { useEffect, useMemo, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { api } from "@/api/client";
import { Button } from "@/ui/Button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/ui/Dialog";
import { Input, Label, TextArea } from "@/ui/Input";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/ui/Select";

const DEFAULT_WORKFLOW_ID = "feature-dev";
const DRAFT_PRESET_ID = "__draft__";

type WorkflowLauncherDialogProps = {
	agentId: string;
	conversationId: string;
	open: boolean;
	onOpenChange: (open: boolean) => void;
	onStarted?: (params: { runId: string; workflowId: string }) => void;
};

type LauncherPrefs = {
	workflowId: string;
	repoPath: string;
	branch: string;
	worktreePath: string;
};

type LauncherPreset = LauncherPrefs & {
	id: string;
	name: string;
};

type LauncherStore = {
	draft: LauncherPrefs;
	presets: LauncherPreset[];
	lastUsedPresetId: string;
};

function storageKey(agentId: string) {
	return `spacebot:workflow-launcher:${agentId}`;
}

function defaultPrefs(): LauncherPrefs {
	return {
		workflowId: DEFAULT_WORKFLOW_ID,
		repoPath: "",
		branch: "",
		worktreePath: "",
	};
}

function loadStore(agentId: string): LauncherStore {
	if (typeof window === "undefined") {
		return {
			draft: defaultPrefs(),
			presets: [],
			lastUsedPresetId: DRAFT_PRESET_ID,
		};
	}

	try {
		const raw = window.localStorage.getItem(storageKey(agentId));
		if (!raw) {
			return {
				draft: defaultPrefs(),
				presets: [],
				lastUsedPresetId: DRAFT_PRESET_ID,
			};
		}

		const parsed = JSON.parse(raw) as
			| Partial<LauncherPrefs>
			| Partial<LauncherStore>;

		// Backward compatibility with the old single-prefs shape.
		if ("workflowId" in parsed || "repoPath" in parsed || "branch" in parsed || "worktreePath" in parsed) {
			const draft = {
				workflowId: parsed.workflowId?.trim() || DEFAULT_WORKFLOW_ID,
				repoPath: parsed.repoPath?.trim() || "",
				branch: parsed.branch?.trim() || "",
				worktreePath: parsed.worktreePath?.trim() || "",
			};
			return {
				draft,
				presets: [],
				lastUsedPresetId: DRAFT_PRESET_ID,
			};
		}

		const draft = parsed.draft ?? defaultPrefs();
		const presets = Array.isArray(parsed.presets)
			? parsed.presets
					.filter((preset): preset is Partial<LauncherPreset> => Boolean(preset))
					.map((preset) => ({
						id: preset.id?.trim() || crypto.randomUUID(),
						name: preset.name?.trim() || "Unnamed preset",
						workflowId: preset.workflowId?.trim() || DEFAULT_WORKFLOW_ID,
						repoPath: preset.repoPath?.trim() || "",
						branch: preset.branch?.trim() || "",
						worktreePath: preset.worktreePath?.trim() || "",
					}))
			: [];

		return {
			draft: {
				workflowId: draft.workflowId?.trim() || DEFAULT_WORKFLOW_ID,
				repoPath: draft.repoPath?.trim() || "",
				branch: draft.branch?.trim() || "",
				worktreePath: draft.worktreePath?.trim() || "",
			},
			presets,
			lastUsedPresetId:
				typeof parsed.lastUsedPresetId === "string" && parsed.lastUsedPresetId.trim()
					? parsed.lastUsedPresetId
					: DRAFT_PRESET_ID,
		};
	} catch {
		return {
			draft: defaultPrefs(),
			presets: [],
			lastUsedPresetId: DRAFT_PRESET_ID,
		};
	}
}

function saveStore(agentId: string, store: LauncherStore) {
	if (typeof window === "undefined") return;
	window.localStorage.setItem(storageKey(agentId), JSON.stringify(store));
}

export function WorkflowLauncherDialog({
	agentId,
	conversationId,
	open,
	onOpenChange,
	onStarted,
}: WorkflowLauncherDialogProps) {
	const store = useMemo(() => loadStore(agentId), [agentId]);

	const [workflowId, setWorkflowId] = useState(store.draft.workflowId);
	const [repoPath, setRepoPath] = useState(store.draft.repoPath);
	const [branch, setBranch] = useState(store.draft.branch);
	const [worktreePath, setWorktreePath] = useState(store.draft.worktreePath);
	const [presetId, setPresetId] = useState(store.lastUsedPresetId);
	const [presetName, setPresetName] = useState("");
	const [presets, setPresets] = useState<LauncherPreset[]>(store.presets);
	const [taskTitle, setTaskTitle] = useState("");
	const [taskBody, setTaskBody] = useState("");
	const [error, setError] = useState<string | null>(null);

	const currentPrefs = useMemo(
		() => ({
			workflowId: workflowId.trim() || DEFAULT_WORKFLOW_ID,
			repoPath: repoPath.trim(),
			branch: branch.trim(),
			worktreePath: worktreePath.trim(),
		}),
		[workflowId, repoPath, branch, worktreePath],
	);

	const persistStore = (next: {
		draft?: LauncherPrefs;
		presets?: LauncherPreset[];
		lastUsedPresetId?: string;
	}) => {
		const nextStore: LauncherStore = {
			draft: next.draft ?? currentPrefs,
			presets: next.presets ?? presets,
			lastUsedPresetId: next.lastUsedPresetId ?? presetId,
		};
		saveStore(agentId, nextStore);
	};

	useEffect(() => {
		if (!open) return;
		const nextStore = loadStore(agentId);
		const selectedPreset =
			nextStore.lastUsedPresetId !== DRAFT_PRESET_ID
				? nextStore.presets.find((preset) => preset.id === nextStore.lastUsedPresetId)
				: null;
		const nextPrefs = selectedPreset ?? nextStore.draft;
		setWorkflowId(nextPrefs.workflowId);
		setRepoPath(nextPrefs.repoPath);
		setBranch(nextPrefs.branch);
		setWorktreePath(nextPrefs.worktreePath);
		setPresetId(selectedPreset?.id ?? DRAFT_PRESET_ID);
		setPresetName(selectedPreset?.name ?? "");
		setPresets(nextStore.presets);
		setTaskTitle("");
		setTaskBody("");
		setError(null);
	}, [agentId, open]);

	useEffect(() => {
		if (!open || presetId !== DRAFT_PRESET_ID) return;
		persistStore({ draft: currentPrefs });
	}, [open, presetId, currentPrefs]);

	const handleSelectPreset = (nextPresetId: string) => {
		setError(null);
		setPresetId(nextPresetId);

		if (nextPresetId === DRAFT_PRESET_ID) {
			const nextStore = loadStore(agentId);
			setWorkflowId(nextStore.draft.workflowId);
			setRepoPath(nextStore.draft.repoPath);
			setBranch(nextStore.draft.branch);
			setWorktreePath(nextStore.draft.worktreePath);
			setPresetName("");
			persistStore({ lastUsedPresetId: DRAFT_PRESET_ID, presets: nextStore.presets, draft: nextStore.draft });
			return;
		}

		const selectedPreset = presets.find((preset) => preset.id === nextPresetId);
		if (!selectedPreset) return;

		setWorkflowId(selectedPreset.workflowId);
		setRepoPath(selectedPreset.repoPath);
		setBranch(selectedPreset.branch);
		setWorktreePath(selectedPreset.worktreePath);
		setPresetName(selectedPreset.name);
		persistStore({ lastUsedPresetId: selectedPreset.id });
	};

	const handleSavePreset = () => {
		const trimmedName = presetName.trim();
		if (!trimmedName) {
			setError("Preset name is required.");
			return;
		}

		setError(null);
		let nextPresetId = presetId;
		const nextPresets = [...presets];
		const payload: LauncherPreset = {
			id: presetId !== DRAFT_PRESET_ID ? presetId : crypto.randomUUID(),
			name: trimmedName,
			...currentPrefs,
		};

		const existingIndex = nextPresets.findIndex((preset) => preset.id === payload.id);
		if (existingIndex >= 0) {
			nextPresets[existingIndex] = payload;
		} else {
			nextPresets.unshift(payload);
		}

		nextPresetId = payload.id;
		setPresetId(nextPresetId);
		setPresets(nextPresets);
		persistStore({
			presets: nextPresets,
			lastUsedPresetId: nextPresetId,
			draft: currentPrefs,
		});
	};

	const handleDeletePreset = () => {
		if (presetId === DRAFT_PRESET_ID) return;
		const nextPresets = presets.filter((preset) => preset.id !== presetId);
		setPresets(nextPresets);
		setPresetId(DRAFT_PRESET_ID);
		setPresetName("");
		persistStore({
			presets: nextPresets,
			lastUsedPresetId: DRAFT_PRESET_ID,
			draft: currentPrefs,
		});
	};

	const mutation = useMutation({
		mutationFn: async () => {
			const trimmedTitle = taskTitle.trim();
			const trimmedBody = taskBody.trim();

			if (!trimmedTitle) {
				throw new Error("Task title is required.");
			}
			if (!currentPrefs.repoPath) {
				throw new Error("Repo path is required.");
			}

			const response = await api.createAntfarmRun({
				request_id: crypto.randomUUID(),
				conversation_id: conversationId,
				workflow_id: currentPrefs.workflowId,
				task_title: trimmedTitle,
				task_body: trimmedBody,
				repo_path: currentPrefs.repoPath,
				branch: currentPrefs.branch || undefined,
				worktree_path: currentPrefs.worktreePath || undefined,
				metadata: {},
			});

			const nextPresets = presets.map((preset) =>
				preset.id === presetId
					? {
							...preset,
							name: preset.name.trim() || "Unnamed preset",
							...currentPrefs,
						}
					: preset,
			);
			setPresets(nextPresets);
			persistStore({
				presets: nextPresets,
				lastUsedPresetId: presetId,
				draft: currentPrefs,
			});

			return response;
		},
		onSuccess: (response) => {
			onOpenChange(false);
			onStarted?.({
				runId: response.run.run_id,
				workflowId: response.run.workflow_id,
			});
		},
		onError: (nextError) => {
			setError(nextError instanceof Error ? nextError.message : "Failed to start workflow.");
		},
	});

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="max-w-xl">
				<DialogHeader>
					<DialogTitle>Run Workflow</DialogTitle>
					<DialogDescription>
						Launch an Antfarm workflow for this conversation. Target repo settings are remembered per agent in this browser.
					</DialogDescription>
				</DialogHeader>

				<div className="flex flex-col gap-4">
					<div className="grid gap-4 sm:grid-cols-[minmax(0,1fr)_160px]">
						<div>
							<Label htmlFor="workflow-preset-select">Project Preset</Label>
							<Select value={presetId} onValueChange={handleSelectPreset}>
								<SelectTrigger id="workflow-preset-select">
									<SelectValue placeholder="Select preset" />
								</SelectTrigger>
								<SelectContent>
									<SelectItem value={DRAFT_PRESET_ID}>Custom draft</SelectItem>
									{presets.map((preset) => (
										<SelectItem key={preset.id} value={preset.id}>
											{preset.name}
										</SelectItem>
									))}
								</SelectContent>
							</Select>
						</div>
						<div>
							<Label htmlFor="workflow-preset-name">Preset Name</Label>
							<Input
								id="workflow-preset-name"
								value={presetName}
								onChange={(event) => setPresetName(event.target.value)}
								placeholder="Target project"
								size="md"
							/>
						</div>
					</div>

					<div className="flex items-center gap-2">
						<Button type="button" variant="outline" size="sm" onClick={handleSavePreset}>
							{presetId === DRAFT_PRESET_ID ? "Save Preset" : "Update Preset"}
						</Button>
						<Button
							type="button"
							variant="ghost"
							size="sm"
							onClick={handleDeletePreset}
							disabled={presetId === DRAFT_PRESET_ID}
						>
							Delete Preset
						</Button>
					</div>

					<div className="grid gap-4 sm:grid-cols-2">
						<div>
							<Label htmlFor="workflow-id">Workflow ID</Label>
							<Input
								id="workflow-id"
								value={workflowId}
								onChange={(event) => setWorkflowId(event.target.value)}
								placeholder={DEFAULT_WORKFLOW_ID}
								size="md"
							/>
						</div>
						<div>
							<Label htmlFor="workflow-branch">Branch</Label>
							<Input
								id="workflow-branch"
								value={branch}
								onChange={(event) => setBranch(event.target.value)}
								placeholder="feature/checkin"
								size="md"
							/>
						</div>
					</div>

					<div>
						<Label htmlFor="workflow-repo-path">Repo Path</Label>
						<Input
							id="workflow-repo-path"
							value={repoPath}
							onChange={(event) => setRepoPath(event.target.value)}
							placeholder="/absolute/path/to/project"
							size="md"
						/>
					</div>

					<div>
						<Label htmlFor="workflow-worktree-path">Worktree Path</Label>
						<Input
							id="workflow-worktree-path"
							value={worktreePath}
							onChange={(event) => setWorktreePath(event.target.value)}
							placeholder="/optional/worktree/path"
							size="md"
						/>
					</div>

					<div>
						<Label htmlFor="workflow-task-title">Task Title</Label>
						<Input
							id="workflow-task-title"
							value={taskTitle}
							onChange={(event) => setTaskTitle(event.target.value)}
							placeholder="Implement user check-in flow"
							size="md"
						/>
					</div>

					<div>
						<Label htmlFor="workflow-task-body">Task Body</Label>
						<TextArea
							id="workflow-task-body"
							value={taskBody}
							onChange={(event) => setTaskBody(event.target.value)}
							placeholder="Acceptance criteria, business rules, QA notes..."
							className="min-h-[140px]"
						/>
					</div>

					{error && (
						<div className="rounded-md border border-red-500/25 bg-red-500/5 px-3 py-2 text-sm text-red-300">
							{error}
						</div>
					)}
				</div>

				<DialogFooter>
					<Button
						variant="outline"
						onClick={() => onOpenChange(false)}
						disabled={mutation.isPending}
					>
						Cancel
					</Button>
					<Button onClick={() => mutation.mutate()} loading={mutation.isPending}>
						Start Workflow
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
