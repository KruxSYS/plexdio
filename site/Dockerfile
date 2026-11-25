# Plexdio website, downloads and update manifests: nginx (unprivileged, uid 101) with the static site baked in.
# Release files come from the plexdio-releases volume (/srv/releases, read-only), written by ./build.sh publish.
# Build context: site/ (see .dockerignore: only pages/, static/, tools/render.py and nginx/ are sent).

# Stage 1: render the pages into the layout, stamp asset links with a content hash, precompress for gzip_static.
FROM python:3.13-alpine@sha256:9ef1a599f863231c3d91e941389bfdc4a2d6070557243d54df1a4e57015b960e AS site
WORKDIR /build
COPY tools/render.py tools/
COPY pages/ pages/
COPY static/ /out/
RUN python3 -I tools/render.py pages /out auto \
    && find /out -type f \( -name '*.html' -o -name '*.css' -o -name '*.js' -o -name '*.svg' -o -name '*.json' \) \
       -exec gzip -9 -k -n {} +

# Stage 2: the server.
FROM nginxinc/nginx-unprivileged:1.30.6-alpine@sha256:f5116ed4606f3eb015dd4a8e28cb9b351476dc0ad4bc6e9f93b19cec130cdf03

# Only ${PLEXDIO_*} variables are substituted in templates; defaults for the .env settings.
ENV NGINX_ENVSUBST_FILTER=^PLEXDIO_ \
    MANIFEST_CACHE_CONTROL=no-cache \
    DOWNLOAD_MAX_AGE=31536000

COPY --chmod=0755 nginx/05-plexdio-env.envsh /docker-entrypoint.d/05-plexdio-env.envsh
COPY nginx/snippets/ /etc/nginx/snippets/
COPY nginx/templates/ /etc/nginx/templates/
COPY --from=site /out/ /usr/share/nginx/html/

USER 101
EXPOSE 8080

HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
    CMD ["wget", "-q", "-O", "/dev/null", "http://127.0.0.1:8080/healthz"]
