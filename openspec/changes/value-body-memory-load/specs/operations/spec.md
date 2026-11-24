## ADDED Requirements

### Requirement: Memory of the value bodies under load
The load job SHALL run two rounds of four parallel value requests, each round on its own new container `app`. On the musl leg, `memory.peak` of `app` after round 2 SHALL stay below `soak.memory_ceiling`, with no OOM kill and no restart. The job SHALL report the round 1 reading without a gate on its peak, OOM counts, or restarts. The `cc-debian13` leg SHALL report these values. A missing or invalid memory value SHALL fail both legs (R30, R31, A26).

#### Scenario: Round 2 below the ceiling
- **GIVEN** the musl leg and four drafts on four machines, each with the parameter `BIG`
- **WHEN** both rounds pass, each round on its own new container `app`
- **AND** `memory.peak` of `app` after round 2 is below 384 MiB
- **THEN** the job passes
- **AND** the summary shows the container ID, the peak, and the run statistic of each round

#### Scenario: Round 1 reports its peak without a gate
- **GIVEN** a round 1 reading of the musl leg with `memory.peak` equal to 384 MiB
- **AND** correct answers in both rounds and a round 2 reading below 384 MiB
- **WHEN** the job checks the memory of `app`
- **THEN** the job does not fail on the round 1 reading
- **AND** the summary shows the peak of round 1

#### Scenario: Round 2 at the ceiling on the musl leg
- **GIVEN** a round 1 reading of the musl leg below 384 MiB and a round 2 reading with `memory.peak` equal to 384 MiB
- **WHEN** the job checks the memory of `app`
- **THEN** the job fails and names the ceiling of 384 MiB
- **AND** the failure names round 2

#### Scenario: Round 1 ends without answers
- **GIVEN** a value leg in which an OOM kill in round 1 ends the four requests without a status
- **WHEN** the step `value-check` runs
- **THEN** the job fails and names round 1 and the request without a status
- **AND** the summary shows the peak and the OOM counts of the round 1 reading

#### Scenario: One container for both rounds
- **GIVEN** a round 1 reading and a round 2 reading with the same container ID
- **WHEN** the job checks the memory of `app`
- **THEN** the job fails and names the container ID

#### Scenario: Peak at the ceiling on the cc-debian13 leg
- **GIVEN** a reading of the value leg of the `cc-debian13` leg with `memory.peak` equal to 384 MiB
- **WHEN** the job checks the memory of `app`
- **THEN** the step of the check reports the ceiling of 384 MiB
- **AND** the leg continues to the step "Require the values from each leg"

#### Scenario: OOM kill in the value leg
- **GIVEN** a round 2 reading of the musl leg with an `oom_kill` count of 1
- **WHEN** the job checks the memory of `app`
- **THEN** the job fails and names the count `oom_kill`

#### Scenario: Missing memory value of the value leg
- **GIVEN** a reading of the value leg in which `memory.peak` is `unavailable`
- **WHEN** the step "Require the values from each leg" runs
- **THEN** that leg fails, also when it is the `cc-debian13` leg
- **AND** the failure names the missing value

### Requirement: Rounds of the value bodies
Each request of a round SHALL go to its own draft, and both rounds SHALL use the same four drafts. In each round, `anon` of `app` SHALL rise by at least 4 × the body length over the baseline of that round. A round 1 body SHALL have 16 MiB − 64 KiB to 16 MiB, SHALL get `422` for `description_en`, and SHALL write nothing. A round 2 body SHALL be the write form, SHALL get `303`, and SHALL increase the `row_version` of its draft (R30, R31, A26).

#### Scenario: Round 1 refuses the full-size bodies
- **GIVEN** four drafts on four machines and four round 1 bodies in the window
- **WHEN** the job sends the four bodies in parallel
- **AND** `anon` of `app` rises by at least 4 × the body length over the baseline of round 1
- **THEN** each request gets `422` with the first field line `E-WEB-008` under `description_en`
- **AND** the `row_version` of each draft equals its value in the state file

#### Scenario: Wrong refusal in round 1
- **GIVEN** a value leg in which one round 1 request gets `422` with a first field line of the field `form`
- **WHEN** the leg checks round 1
- **THEN** the leg fails before round 2 and names the field `form`

#### Scenario: Round 2 writes the write forms
- **GIVEN** the four drafts after round 1 and four round 2 bodies
- **AND** each round 2 body is the write form, the pinned valid edit form of the fixture
- **WHEN** the job sends the four bodies in parallel
- **AND** `anon` of `app` rises by at least 4 × the body length over the baseline of round 2
- **THEN** each request gets `303` with the `Location` of the parameter page of its key
- **AND** the `row_version` of each draft is larger than before the leg

#### Scenario: Wrong redirect in the pre-check
- **GIVEN** a pre-check in which the round 2 body to `LOAD-V5` gets `303` with a `Location` of another page
- **WHEN** the leg checks the pre-check
- **THEN** the leg fails before it replaces `app` and names the `Location`

#### Scenario: Bodies that do not reach the barrier
- **GIVEN** a server that reads only one value body at a time
- **WHEN** the job sends the four round 1 bodies in parallel
- **THEN** the job fails and names the request that did not reach the barrier

#### Scenario: Bodies that do not overlap in the server
- **GIVEN** four requests of a round at the barrier and a rise of `anon` of `app` below 4 × the body length
- **WHEN** the job checks the reading of the barrier
- **THEN** the job fails with the text "the bodies did not overlap"
- **AND** the failure names the round

#### Scenario: Written round 1 body
- **GIVEN** a value leg in which one round 1 request gets `303`
- **WHEN** the leg checks round 1
- **THEN** the leg fails before round 2 and names the status of that request

#### Scenario: Refused round 2 request
- **GIVEN** a value leg in which one round 2 request gets `409`
- **WHEN** the job checks the answers
- **THEN** the job fails and names the status of that request

#### Scenario: Redirect to the login page
- **GIVEN** a value leg in which one round 2 request gets `303` with the `Location` `/login`
- **WHEN** the job checks the answers
- **THEN** the job fails and names the `Location` of that request

## MODIFIED Requirements

### Requirement: Memory of the container app under load
The load job SHALL read the memory of the container `app` after the timings, after the flood, and six times in the value leg. The container SHALL stay under its memory limit of 512 MiB, with no OOM kill and no restart. The `value-round-1` reading SHALL only report these bounds. A failed bound of another reading SHALL fail the job. A missing memory value of any reading SHALL fail the job (R30, R31, A26).

#### Scenario: Memory limit exceeded
- **GIVEN** a load run in which the kernel kills a process of `app` for its memory limit
- **AND** the reading of that kill is not the `value-round-1` reading
- **WHEN** the job checks the memory of `app`
- **THEN** the job fails and names the OOM kill

#### Scenario: Restart of the container
- **GIVEN** a load run in which the container `app` has a restart count of 1
- **AND** the reading of that count is not the `value-round-1` reading
- **WHEN** the job checks the memory of `app`
- **THEN** the job fails and names the restart

#### Scenario: Peak at the memory limit
- **GIVEN** a memory reading other than the `value-round-1` reading with `memory.peak` equal to 512 MiB
- **WHEN** the job checks the memory of `app`
- **THEN** the job fails and names the limit of 512 MiB

#### Scenario: Missing memory value
- **GIVEN** a memory reading in which `memory.peak` is `unavailable`, also the `value-round-1` reading
- **WHEN** the job checks the memory of `app`
- **THEN** the job fails and names the missing value

#### Scenario: Round 1 reading at the memory limit
- **GIVEN** a `value-round-1` reading with `memory.peak` equal to 512 MiB
- **AND** that reading has an `oom_kill` count of 1 and a restart count of 1
- **WHEN** the job checks the memory of `app`
- **THEN** the check of the memory does not fail on that reading
- **AND** the job reports the peak, the count `oom_kill`, and the restart count of that reading
