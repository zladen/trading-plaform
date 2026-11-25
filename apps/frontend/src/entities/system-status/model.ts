/**
 * System status model.
 *
 * The dashboard shows the state of the backend process and of each
 * infrastructure dependency it uses. Those values come from `/health`,
 * `/ready` and `/info`; the entity layer maps them into one view model so
 * widgets and pages do not duplicate mapping logic.
 */

import type { HealthPayload, InfoPayload, ReadyPayload } from "@/shared/api";

/** Health of a single component shown on the dashboard. */
export type ComponentState = "up" | "down" | "degraded" | "unknown";

/** Which component a status refers to. */
export type ComponentId = "backend" | "postgres" | "redis" | "websocket";

/** Human readable label for a component. */
export const COMPONENT_LABELS: Record<ComponentId, string> = {
	backend: "Backend",
	postgres: "PostgreSQL",
	redis: "Redis",
	websocket: "WebSocket",
};

export interface ComponentStatus {
	id: ComponentId;
	label: string;
	state: ComponentState;
	/** Optional explanation, for example a probe error or a latency. */
	detail: string | null;
}

/** Overall system state derived from every component. */
export type SystemState = "operational" | "degraded" | "unavailable";

export interface BackendInfo {
	name: string;
	version: string;
	environment: string;
	rustVersion: string | null;
	startedAt: string;
	uptimeSeconds: number;
	websocketPath: string;
}

export interface SystemSnapshot {
	backend: ComponentStatus;
	dependencies: ComponentStatus[];
	info: BackendInfo;
	checkedAt: string;
	requestId: string;
}

const DEPENDENCY_IDS: Record<string, ComponentId> = {
	postgres: "postgres",
	redis: "redis",
};

function dependencyLabel(dependency: string): string {
	const known = DEPENDENCY_IDS[dependency];

	return known === undefined ? dependency : COMPONENT_LABELS[known];
}

function dependencyId(dependency: string): ComponentId {
	return DEPENDENCY_IDS[dependency] ?? "backend";
}

/**
 * Maps a readiness report into dependency statuses.
 *
 * An unknown dependency name is surfaced with its raw identifier instead of
 * being dropped, so a newly added dependency is visible before the UI learns
 * about it.
 */
export function mapReadiness(readiness: ReadyPayload): ComponentStatus[] {
	return readiness.checks.map((check) => {
		const state: ComponentState = check.status === "up" ? "up" : "down";

		const detail =
			state === "up"
				? `${check.latency_ms} ms`
				: (check.error ?? "probe reported the dependency as unavailable");

		return {
			id: dependencyId(check.dependency),
			label: dependencyLabel(check.dependency),
			state,
			detail,
		};
	});
}

/** Maps the backend health payload into a component status. */
export function mapHealth(health: HealthPayload): ComponentStatus {
	return {
		id: "backend",
		label: COMPONENT_LABELS.backend,
		state: health.status === "up" ? "up" : "down",
		detail: health.status === "up" ? null : `reported status: ${health.status}`,
	};
}

/** Maps the info payload into the display model. */
export function mapInfo(info: InfoPayload): BackendInfo {
	return {
		name: info.name,
		version: info.version,
		environment: info.environment,
		rustVersion: info.rust_version ?? null,
		startedAt: info.started_at,
		uptimeSeconds: info.uptime_seconds,
		websocketPath: info.websocket_path,
	};
}

/**
 * Builds a complete snapshot from the three API responses.
 *
 * The backend is reported `down` when any dependency is down, because the API
 * can answer HTTP while being unable to serve real workload. That distinction
 * is what makes `/ready` different from `/health`.
 */
export function buildSnapshot(input: {
	health: HealthPayload;
	readiness: ReadyPayload;
	info: InfoPayload;
	requestId: string;
}): SystemSnapshot {
	const dependencies = mapReadiness(input.readiness);
	const backendUp = input.health.status === "up";
	const degraded = backendUp && dependencies.some((dependency) => dependency.state !== "up");

	const backend: ComponentStatus = {
		id: "backend",
		label: COMPONENT_LABELS.backend,
		state: !backendUp ? "down" : degraded ? "degraded" : "up",
		detail: degraded ? "serving, but a dependency is unavailable" : null,
	};

	return {
		backend,
		dependencies,
		info: mapInfo(input.info),
		checkedAt: input.readiness.checked_at,
		requestId: input.requestId,
	};
}

/** Reduces a snapshot to one headline state. */
export function deriveSystemState(snapshot: SystemSnapshot): SystemState {
	const states = [snapshot.backend.state, ...snapshot.dependencies.map((item) => item.state)];

	if (states.every((state) => state === "up")) {
		return "operational";
	}

	if (states.every((state) => state === "down")) {
		return "unavailable";
	}

	return "degraded";
}
