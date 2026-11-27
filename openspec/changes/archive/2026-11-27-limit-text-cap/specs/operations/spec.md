## ADDED Requirements

### Requirement: Largest anon of round 2
In round 2, the value leg SHALL keep its reads of `anon` of `app` after the first read at or above the overlap threshold. It SHALL stop them at the end of the record window or at the barrier deadline, and then release the barrier. `load-value.json` and the job summary SHALL record the largest read and its margin. This record SHALL NOT change the result of the overlap check (R30, R31).

#### Scenario: Round 2 records the largest anon
- **GIVEN** the musl leg and four round 2 requests at the barrier
- **AND** a first read of `anon` at the threshold and a larger read in the record window
- **WHEN** the value leg ends the record window and releases the barrier
- **THEN** `load-value.json` holds the larger read and its margin over the sum of the reference and the four body lengths
- **AND** the job summary shows both values for round 2

#### Scenario: Record window ends before the barrier deadline
- **GIVEN** a round 2 with a first read at the threshold more than one record window before the barrier deadline
- **WHEN** the record window ends
- **THEN** the value leg stops the reads and releases the barrier at the end of the record window
- **AND** the record holds no read after the end of the record window

#### Scenario: The record does not change the overlap check
- **GIVEN** a round 2 with a first read of `anon` at the threshold and smaller reads after it
- **WHEN** the step `value-check` runs
- **THEN** the overlap check of round 2 passes
- **AND** the recorded largest read equals the first read

#### Scenario: Barrier deadline in the record window
- **GIVEN** a round 2 with a first read at the threshold less than one record window before the barrier deadline
- **WHEN** the barrier deadline arrives
- **THEN** the value leg stops the reads and releases the barrier
- **AND** the overlap check of round 2 passes
- **AND** the record holds the largest read before the deadline

#### Scenario: Failed read in the record window
- **GIVEN** a round 2 with a first read at the threshold
- **AND** a later read in the record window that fails or times out
- **AND** a larger valid read after the failed read
- **WHEN** the record window ends
- **THEN** the value leg releases the barrier
- **AND** the overlap check of round 2 passes
- **AND** the record holds the larger valid read

#### Scenario: Round 2 without a crossing
- **GIVEN** a round 2 with valid reads of `anon` before the barrier deadline, each below the threshold
- **WHEN** the step `value-check` runs
- **THEN** the only overlap failure of the leg is "the bodies did not overlap"
- **AND** the largest read and its margin are `null`

#### Scenario: Round 2 without a valid read
- **GIVEN** a round 2 with no valid read of `anon` before the barrier deadline
- **WHEN** the step `value-check` runs
- **THEN** the leg reports the failure "round 2 barrier_anon is missing or invalid"
- **AND** the largest read and its margin are `null`

#### Scenario: Round 2 with a missed barrier
- **GIVEN** a round 2 in which the request `LOAD-V1` did not reach the barrier
- **WHEN** the step `value-check` runs
- **THEN** the leg reports the failure "round 2: request LOAD-V1 did not reach the barrier"
- **AND** the largest read and its margin are `null`

#### Scenario: Shape of the values without a crossing
- **GIVEN** a `load-value.json` in which round 2 has `null` for the largest read and its margin
- **WHEN** the step "Require the values from each leg" runs
- **THEN** the step accepts both values

#### Scenario: Round 1 keeps the release at the first read
- **GIVEN** a round 1 with a first read of `anon` at the threshold
- **WHEN** the value leg reads that value
- **THEN** the value leg releases the barrier with no record window
- **AND** `load-value.json` holds no largest read for round 1

#### Scenario: Missing largest read of round 2
- **GIVEN** a round 2 batch `load-value-answers-2.json` with a `barrier_anon` that gives an `overlap_margin` of 0 or more and no largest read
- **WHEN** the step `value-check` runs
- **THEN** the `load-value.json` that the step writes holds a failure that names the missing value
- **AND** the job summary shows that failure
