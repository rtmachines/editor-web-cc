# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e
# Image recipe-pgtools: the services backup and walarchive (design D9, ADR-0064, ADR-0066).
# The base image is the image of the database, so pg_dump has the version of the server.
# Digest source: docker buildx imagetools inspect postgres:16.15-bookworm
FROM postgres:16.15-bookworm@sha256:d81f7b51697014367236d07ec58b54153fc28f828b72bf247fe5c7487520bcb0

LABEL org.opencontainers.image.title="recipe-pgtools" \
      org.opencontainers.image.description="Backup, restore test, and WAL archive tools of the recipe editor"

# supercronic v0.2.51. Upstream publishes SHA-1 only. This SHA-256 comes from our own download.
# The build stops when the checksum differs. The file gets mode 0755 only after that check.
ADD --checksum=sha256:cc232108374e8e2b537045f2d5eaa1229f619db314c3ba6a14d38b35dc0aa123 \
    --chmod=0755 \
    https://github.com/aptible/supercronic/releases/download/v0.2.51/supercronic-linux-amd64 \
    /usr/local/bin/supercronic

COPY --chmod=0755 bin/ /usr/local/bin/
# Create the directory first so COPY keeps its search permission for uid 999.
RUN mkdir -p -m 0755 /etc/pgtools
COPY --chmod=0644 crontab.backup crontab.walarchive /etc/pgtools/
COPY --chmod=0644 restore-code.sql restore-code.expected restore-roles.sql restore-roles.expected /etc/pgtools/
COPY --chmod=0644 lib/globals-grammar.sh lib/restore-steps.sh lib/wal-identity.sh /etc/pgtools/
# The named build context db is deploy/db. It holds the shared client rules of the cluster (ADR-0103 (l)).
COPY --chmod=0644 --from=db cluster-access.sh /etc/pgtools/cluster-access.sh

# The base image sets the entry of the database. This image runs a script of bin/ instead.
ENTRYPOINT []
STOPSIGNAL SIGTERM
USER 999:999
CMD ["backup-entry"]
