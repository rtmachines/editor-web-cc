# ADR-0064: Images and their distribution

Status: Accepted
Date: 2026-10-23
Deciders: project owner

## Context

`reliability.md` section 7.1 names the image `recipe-server` for `app` and `migrate`, and the same base image as `db` for `backup` and `walarchive`. No document names a base image, a build target, or a way to bring an image to the plant. Step P4.1 builds the images, and the plan found these facts and gaps:

- The distroless project updates only its Debian 13 images. Its Debian 12 tags are deprecated.
- In the release graph of `recipe-server`, only `ring` builds C code. The static files are in the binary, so the runtime image needs no other file.
- `rust-toolchain.toml` names the channel `stable` and no version. A build can then get another toolchain than `rust-version` of the workspace.
- Section 7.10 step 2 says "Pull the new images". The plant host can have no registry and no internet access.
- A changed image file runs with every secret of its service. No document gives a check of the file.
- A build context with a deny list can copy a credential file into a layer of the build cache.
- The panic policy needs `panic = "unwind"` (section 2.4 rule 1). The unwind of a binary for the musl target needs a proof.
- The allocator of musl can be slower under the login flood of section 6.
- The host sets `core.autocrlf=true`. A script or a `Caddyfile` with CRLF line ends can fail in a Linux container.

## Decision drivers

- The runtime image holds no shell and no package manager, and it runs as a user that is not root (R04, R34).
- A release reaches a plant host without a registry (R30).
- Each image of the project and each external image has a fixed identity (R34).
- The soak ceiling and the p99 bound of `reliability.md` section 6 hold.

## Considered options

### Option A: `distroless/static-debian13:nonroot`, the musl target, and image files

- Pros: an image of a few MiB plus the binary. No libc couples the build to the runtime. The files of `docker save` need no registry.
- Cons: the musl target needs `musl-tools` in the build stage. The allocator of musl can be slower.

### Option B: `distroless/cc-debian13:nonroot` and the glibc target

- Pros: the glibc allocator, and no extra build package.
- Cons: a larger image. The build suite must not be newer than the runtime suite.

### Option C: A registry at the plant

- Pros: `docker compose pull` works as section 7.10 says.
- Cons: the plant must run and secure one more service.

## Decision

The project uses **Option A** for the image and the image files of item (h) instead of Option C.

Runtime image:

- (a) The runtime stage is `gcr.io/distroless/static-debian13:nonroot`, pinned by digest. The user is `65532:65532`.
- (b) The binary is `/usr/local/bin/recipe-server`, owned by `root` with the mode `0755`. `ENTRYPOINT` is `["/usr/local/bin/recipe-server"]`, and `CMD` is `["serve"]`.
- (c) The Dockerfile has no `HEALTHCHECK`. The runs of `migrate` and `create-admin` then get none. `compose.yaml` gives the healthcheck of `app` (ADR-0059).

Build:

- (d) The build target is `x86_64-unknown-linux-musl`. The build image is `rust:1.99.1-slim-trixie` with `musl-tools`, pinned by digest. `RUSTUP_TOOLCHAIN=1.99.1` overrides the channel of `rust-toolchain.toml`.
- (e) `cargo-chef` 0.1.78 caches the dependencies in the stages `chef`, `planner`, and `builder`. The builder runs `cargo build --release --locked`.
- (f) `.dockerignore` is an allow list of the build inputs. A file that the list does not name stays out of the build context.
- (g) `.gitattributes` sets `eol=lf` for `Caddyfile*`, `Dockerfile`, `crontab.*`, `deploy/pgtools/bin/*`, and `deploy/pgtools/tests/*`.

Names and distribution:

- (h) The images of the project are `recipe-server:<version>` and `recipe-pgtools:<version>`. ADR-0066 gives the content of `recipe-pgtools`.
- (i) `compose.yaml` names them with `image:` and `${RECIPE_VERSION}`. `compose.build.yaml` adds `build:` for dev and CI. Each external image has a pinned digest.
- (j) A release moves the images as files of `docker save`. The plant loads them with `docker load`. Section 7.10 step 2 changes from "pull" to "load".
- (k) The release notes give the SHA-256 of each image file and the image ID of each image. The runbook compares the SHA-256 before `docker load` and the image ID after it.
- (l) `docs/ops/runbook.md` now exists. The sentence "Until then, this section is the runbook" of section 7.10 goes away.

Proof and fallback:

- (m) A CI job `musl-unwind` runs the panic tests of `recipe-web` for the musl target. It proves that a panic unwinds in a musl binary.
- (n) The Compose QA of step P4.1 runs one `oha` login flood on the musl image and on a build with `cc-debian13`. It records the logins per second, the p99 latency, and `memory.peak` of the container.
- (o) The fallback to Option B applies when the flood of the QA or the soak of step P4.2 fails. A failure is a value above the soak ceiling or the p99 bound of section 6. The rule adds no number.
- (p) A switch to the fallback is a new ADR that names the new base image.

## Consequences

### Positive

- The runtime image holds the binary, certificates, time zones, and a user file. It holds no shell.
- A plant host without a registry gets a release with a check of each file.
- The build uses one toolchain version in every stage.

### Negative

- The build needs a C compiler for the musl target.
- The release process writes and checks image files.
- The operator cannot open a shell in the `app` container.

### Risks and mitigations

- Risk: the allocator of musl fails the soak. Mitigation: items (n) and (o) measure both builds and give the rule of the switch.
- Risk: a file of the build context holds a credential. Mitigation: the allow list of item (f), and CI writes its secrets outside the build context.

## Related

- ADR-0020 (seven services), ADR-0059 (healthcheck of `app`), ADR-0066 (content of `recipe-pgtools`)
- `docs/architecture/reliability.md` section 2.4 rule 1, section 6 (rows "Load" and "Soak"), and section 7.1
- `docs/architecture/reliability.md` section 7.10 (step 2 and the sentence on the runbook)
- `overview.md` section 7: R04, R30, R34
