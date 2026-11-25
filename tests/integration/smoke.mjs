#!/usr/bin/env node
/**
 * End-to-end smoke test against a running stack.
 *
 * Verifies what a deployment actually exposes: the unified envelope, the real
 * readiness aggregation, the generated OpenAPI document, the WebSocket handshake
 * and protocol frames, and that the frontend bundle is served.
 *
 * Usage:
 *   node tests/integration/smoke.mjs
 *   BASE_URL=http://127.0.0.1:8080 FRONTEND_URL=http://127.0.0.1:8081 \
 *     node tests/integration/smoke.mjs
 *
 * Exits non-zero on the first failed assertion. Requires Node 22+ (global
 * `fetch` and `WebSocket`); no third-party dependency is used.
 */

const BASE_URL = (process.env.BASE_URL ?? "http://127.0.0.1:8080").replace(/\/$/, "");
const FRONTEND_URL = (process.env.FRONTEND_URL ?? "http://127.0.0.1:8081").replace(/\/$/, "");
const WS_URL = `${BASE_URL.replace(/^http/, "ws")}/api/v1/ws`;

let checks = 0;
const failures = [];

function check(name, condition, detail = "") {
	checks += 1;
	if (condition) {
		console.log(`  ok   ${name}`);
		return;
	}

	console.error(`  FAIL ${name}${detail ? ` — ${detail}` : ""}`);
	failures.push(name);
}

function assertEqual(name, actual, expected) {
	check(name, actual === expected, `expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
}

async function getJson(path) {
	const response = await fetch(`${BASE_URL}${path}`, {
		headers: { accept: "application/json", "x-request-id": "smoke-test" },
	});
	const body = await response.json();
	return { response, body };
}

function hasEnvelope(body) {
	if (typeof body !== "object" || body === null) return false;
	if (typeof body.request_id !== "string" || body.request_id.length === 0) return false;

	if (body.success === true) return typeof body.data === "object" && body.data !== null;
	if (body.success === false) {
		return (
			typeof body.error === "object" &&
			body.error !== null &&
			typeof body.error.code === "string" &&
			typeof body.error.message === "string"
		);
	}

	return false;
}

async function checkHealth() {
	console.log("\nGET /api/v1/health");
	const { response, body } = await getJson("/api/v1/health");

	assertEqual("status is 200", response.status, 200);
	check("unified success envelope", hasEnvelope(body));
	assertEqual("data.status is up", body.data?.status, "up");
	check("uptime is a number", typeof body.data?.uptime_seconds === "number");
	check("checked_at is an ISO timestamp", !Number.isNaN(Date.parse(body.data?.checked_at ?? "")));
	assertEqual("caller request id is echoed", response.headers.get("x-request-id"), "smoke-test");
}

async function checkReadiness() {
	console.log("\nGET /api/v1/ready");
	const { response, body } = await getJson("/api/v1/ready");

	check("status is 200 or 503", response.status === 200 || response.status === 503, `got ${response.status}`);
	check("unified envelope even on 503", hasEnvelope(body));
	check("ready is a boolean", typeof body.data?.ready === "boolean");
	assertEqual("status matches ready", body.data?.status === "ready", body.data?.ready === true);
	assertEqual("http status matches ready", response.status === 200, body.data?.ready === true);

	const dependencies = (body.data?.checks ?? []).map((entry) => entry.dependency).sort();
	check(
		"reports PostgreSQL and Redis",
		dependencies.includes("postgres") && dependencies.includes("redis"),
		`got ${JSON.stringify(dependencies)}`,
	);

	for (const entry of body.data?.checks ?? []) {
		check(
			`${entry.dependency} check is up`,
			entry.status === "up",
			`status ${entry.status}${entry.error ? `: ${entry.error}` : ""}`,
		);
		check(`${entry.dependency} reports latency`, typeof entry.latency_ms === "number");
	}
}

async function checkInfo() {
	console.log("\nGET /api/v1/info");
	const { response, body } = await getJson("/api/v1/info");

	assertEqual("status is 200", response.status, 200);
	check("unified success envelope", hasEnvelope(body));
	check("name is present", typeof body.data?.name === "string" && body.data.name.length > 0);
	check("version is present", typeof body.data?.version === "string");
	check("environment is present", typeof body.data?.environment === "string");
	assertEqual("websocket path", body.data?.websocket_path, "/api/v1/ws");
	check("started_at is an ISO timestamp", !Number.isNaN(Date.parse(body.data?.started_at ?? "")));
}

async function checkOpenApi() {
	console.log("\nGET /api/v1/openapi.json");
	const { response, body } = await getJson("/api/v1/openapi.json");

	assertEqual("status is 200", response.status, 200);
	check("declares OpenAPI 3.x", typeof body.openapi === "string" && body.openapi.startsWith("3."));
	check("has info.title", typeof body.info?.title === "string");
	check("has info.version", typeof body.info?.version === "string");

	const paths = Object.keys(body.paths ?? {});
	for (const path of ["/api/v1/health", "/api/v1/ready", "/api/v1/info", "/api/v1/openapi.json"]) {
		check(`documents ${path}`, paths.includes(path), `paths: ${JSON.stringify(paths)}`);
	}
}

async function checkErrors() {
	console.log("\nError handling");
	const response = await fetch(`${BASE_URL}/api/v1/does-not-exist`, {
		headers: { accept: "application/json", "x-request-id": "smoke-404" },
	});
	const body = await response.json();

	assertEqual("unknown route is 404", response.status, 404);
	check("error uses the unified envelope", hasEnvelope(body));
	check("error carries a stable code", typeof body.error?.code === "string" && body.error.code.length > 0);
	// The header and the body must agree, otherwise correlating a failure by
	// `x-request-id` silently loses the caller's identifier.
	assertEqual("404 body echoes the caller request id", body.request_id, "smoke-404");
	assertEqual(
		"404 header echoes the caller request id",
		response.headers.get("x-request-id"),
		"smoke-404",
	);

	const post = await fetch(`${BASE_URL}/api/v1/health`, {
		method: "POST",
		headers: { "x-request-id": "smoke-405" },
	});
	const postBody = await post.json();
	assertEqual("wrong method is 405", post.status, 405);
	assertEqual("405 body echoes the caller request id", postBody.request_id, "smoke-405");
}

async function checkWebSocket() {
	console.log("\nWebSocket /api/v1/ws");

	const frames = await new Promise((resolve, reject) => {
		const received = [];
		const socket = new WebSocket(WS_URL);
		const timer = setTimeout(() => {
			socket.close();
			reject(new Error("timed out waiting for the `connected` frame"));
		}, 5000);

		socket.addEventListener("message", (event) => {
			try {
				received.push(JSON.parse(event.data));
			} catch {
				clearTimeout(timer);
				socket.close();
				reject(new Error(`non-JSON frame: ${String(event.data)}`));
				return;
			}

			// Ask for a pong as soon as the server greets us.
			if (received.length === 1) {
				socket.send(JSON.stringify({ type: "ping", payload: "smoke" }));
			}

			if (received.length === 2) {
				clearTimeout(timer);
				socket.close();
				resolve(received);
			}
		});

		socket.addEventListener("error", () => {
			clearTimeout(timer);
			reject(new Error(`could not connect to ${WS_URL}`));
		});
	});

	const [connected, reply] = frames;

	assertEqual("first frame is connected", connected.type, "connected");
	assertEqual("protocol version", connected.protocol_version, 1);
	check("connection id is present", typeof connected.connection_id === "string");
	assertEqual("ping is answered with pong", reply.type, "pong");
	assertEqual("pong payload is echoed", reply.payload, "smoke");
}

async function checkFrontend() {
	console.log(`\nFrontend ${FRONTEND_URL}`);

	let response;
	try {
		response = await fetch(FRONTEND_URL);
	} catch (error) {
		check("frontend is reachable", false, String(error));
		return;
	}

	assertEqual("index is served", response.status, 200);
	const html = await response.text();
	check("html has a root element", /<div id="root">/.test(html));
	check("html references a built asset", /<script[^>]+src="\/assets\//.test(html));

	const assetPath = html.match(/src="(\/assets\/[^"]+\.js)"/)?.[1];
	if (assetPath) {
		const asset = await fetch(`${FRONTEND_URL}${assetPath}`);
		assertEqual("the referenced bundle exists", asset.status, 200);
	}

	// The SPA shell must be returned for an unknown path, not a 404.
	const spa = await fetch(`${FRONTEND_URL}/dashboard`);
	assertEqual("unknown path falls back to the shell", spa.status, 200);

	const proxy = await fetch(`${FRONTEND_URL}/api/v1/health`);
	assertEqual("the API is proxied through the frontend", proxy.status, 200);
	const proxied = await proxy.json();
	check("proxied response keeps the envelope", hasEnvelope(proxied));
}

async function main() {
	console.log(`Smoke test against backend ${BASE_URL}`);

	try {
		await checkHealth();
		await checkReadiness();
		await checkInfo();
		await checkOpenApi();
		await checkErrors();
		await checkWebSocket();
		await checkFrontend();
	} catch (error) {
		console.error(`\nAborted: ${error instanceof Error ? error.message : String(error)}`);
		process.exitCode = 1;
		return;
	}

	console.log(`\n${checks - failures.length}/${checks} checks passed`);

	if (failures.length > 0) {
		console.error(`Failed: ${failures.join(", ")}`);
		process.exitCode = 1;
	}
}

await main();
