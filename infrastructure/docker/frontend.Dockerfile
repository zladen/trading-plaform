# syntax=docker/dockerfile:1

# Frontend image: build the Vite bundle, then serve the static files with nginx,
# which also proxies `/api` (including the WebSocket upgrade) to the backend.
#
# The build context is the repository root so the pnpm workspace manifest and
# lockfile are available for a reproducible, frozen install.

FROM node:24.22.3-trixie-slim AS builder

WORKDIR /build

ENV PNPM_HOME=/pnpm \
    PATH=/pnpm:$PATH \
    CI=true

# Match the `packageManager` field in the root manifest.
RUN npm install --global pnpm@12.16.1 \
    && pnpm config set store-dir /pnpm/store

# Only the files that affect dependency resolution are copied first, so the
# dependency layer stays cached when application source changes.
COPY package.json pnpm-workspace.yaml pnpm-lock.yaml ./
COPY apps/frontend/package.json ./apps/frontend/package.json

RUN --mount=type=cache,id=pnpm-store,target=/pnpm/store \
    pnpm install --frozen-lockfile --filter @platform/frontend...

COPY apps/frontend ./apps/frontend

# Type checking is part of `build`, so a type error fails the image.
RUN pnpm --filter @platform/frontend build

FROM nginx:1.30.8-alpine3.24-slim AS runtime

# Replace the default configuration: it listens on 8080 as a non-root user and
# proxies `/api` to the backend service.
RUN rm -f /etc/nginx/conf.d/default.conf \
    && rm -f /var/log/nginx/access.log /var/log/nginx/error.log \
    && ln -s /dev/stdout /var/log/nginx/access.log \
    && ln -s /dev/stderr /var/log/nginx/error.log \
    && mkdir -p /var/cache/nginx/client_temp /var/cache/nginx/proxy_temp \
    && chown -R nginx:nginx /var/cache/nginx \
    && sed -i 's#^pid .*#pid /tmp/nginx.pid;#' /etc/nginx/nginx.conf

COPY infrastructure/docker/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /build/apps/frontend/dist /usr/share/nginx/html

# The alpine nginx image already provides the `nginx` user (uid 101).
USER nginx

EXPOSE 8080

HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=5 \
    CMD ["wget", "--quiet", "--tries=1", "--spider", "http://127.0.0.1:8080/healthz"]

ENTRYPOINT ["nginx", "-g", "daemon off;"]
