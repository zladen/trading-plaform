/// <reference types="vite/client" />

interface ImportMetaEnv {
	/** Base URL of the backend API. Defaults to the current origin. */
	readonly VITE_API_URL?: string;
	/** Target used by the Vite dev proxy for `/api` requests. */
	readonly VITE_DEV_API_TARGET?: string;
}

interface ImportMeta {
	readonly env: ImportMetaEnv;
}
