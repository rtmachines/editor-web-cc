# ADR-0198: Release notes in the body of the GitHub release

Status: Accepted. Amends ADR-0064 (k) (the place of the release notes)
Date: 2026-11-19
Deciders: project owner

## Context

ADR-0064 (k) says that the release notes give the SHA-256 of each image file and the image ID of each image.
No ADR and no document of `docs/architecture/` names the place of the release notes.

These facts shape the decision:

- `reliability.md` 7.10 step 1 and the runbook section "Upgrade" tell the operator to read the release notes.
- ADR-0097 (h3), ADR-0103 (p), ADR-0105 (n), and ADR-0151 (n) put upgrade facts into the release notes.
- In ADR-0174 (aj), the operator reads the new release tag from the release notes in step 1 of `reliability.md` 7.10.
- The bodies of the GitHub releases `v0.7.0`, `v0.7.1`, and `v0.8.0` hold the `CHANGELOG.md` section of their release.
- The body of `v0.8.0` also holds the SHA-256 of each `docker save` file and the image ID of each image.
- Step 7 of the section "Release" of `CONTRIBUTING.md` puts these values into the body. `CHANGELOG.md` does not hold them.
- The GitHub releases up to `v0.7.1` hold no SHA-256 and no image ID.

The project owner decided on 2026-11-19 that these values go into the body of the GitHub release after the tag.

## Decision drivers

- The operator reads one place before an upgrade and before a load of the images (R30, R34).
- Each existing mention of the release notes stays true.
- The runbook uses one term for one meaning.

## Considered options

### Option A: Define the release notes as the body of the GitHub release

One new item gives the place. The texts of the other ADRs stay.

### Option B: Replace the term in each document

An accepted ADR changes only by its Status line and an "Amended by" note. Option B cannot change ADR-0064, ADR-0097, ADR-0103, ADR-0105, ADR-0151, or ADR-0174.

## Decision

The project uses Option A.

- (a) The release notes of a release are the body of the GitHub release of its Git tag `vX.Y.Z`.
- (b) The body holds the `CHANGELOG.md` section of the release.
- (c) From release 0.8.0, the body also holds the SHA-256 of each `docker save` file and the image ID of each image. The maintainer adds them after the build from the tag (ADR-0064 (i), (j)). This item amends ADR-0064 (k).
- (d) The release notes of a release up to 0.7.1 hold no such value. The runbook section "Load the images" stops before `docker load` for such a release. The runbook section "Keep the images for a rollback" skips the image ID comparison for such a release. The operator records that skip in the change log of the site.
- (e) The runbook uses the term "release notes" for this body. The section "Load the images" gives this definition once.

## Consequences

### Positive

- Each mention of the release notes in the ADRs and in `reliability.md` 7.10 names one place.
- The runbook compares the SHA-256 and the image IDs with one source.

### Negative

- A site without access to GitHub needs a copy of the release notes. The runbook section "Copy the backups off site" keeps one with the image files.
- A site cannot load a deleted image of a release up to 0.7.1 again. A rollback to that release then stops.
  The runbook section "Keep the images for a rollback" tells the operator to keep these images on the host.

### Risks and mitigations

- Risk: a later edit of the body changes a value. Mitigation: the site keeps its own copy of the release notes with the image files.

## Related

- ADR-0064 (i), (j), and (k), images and their distribution.
- ADR-0097 (h3), ADR-0103 (p), ADR-0105 (n), and ADR-0151 (n), upgrade facts in the release notes.
- ADR-0174 (aj), the new release tag from the release notes.
- `docs/architecture/reliability.md` section 7.10, step 1.
- `docs/ops/runbook.md`, sections "Load the images", "Keep the images for a rollback", and "Upgrade".
- `CONTRIBUTING.md`, section "Release".
- `overview.md` §7: R30, R34.
