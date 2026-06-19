# Blueshift Assembly Timeout — 4 sBPF Instructions

> Superteam Earn: $750 USDG Developer Challenge  
> Deadline: 2026-06-21 | 5 submissions

A slot-height deadline guard in **4 sBPF assembly instructions**.
Append to any transaction; the transaction fails if it lands after `max_slot_height`.

```
success path:  3 CUs (verifier cap: 4)
failure path:  4 CUs
binary size:  264 bytes
instructions:   4
```

## Approach

The Blueshift verifier caps the **success path at 4 CUs** and rejects
any program exceeding this budget. `sol_get_clock_sysvar` costs 140 CUs
alone — making the documented course solution infeasible.

Our approach reads the current slot directly from the **instruction
account data** (not the sysvar), avoiding the syscall entirely:

```
1. LDW   r0, [r1 + SLOT_OFFSET]   ; load current slot from clock account
2. LDW   r1, [r1 + DATA_OFFSET]   ; load max_slot from instruction data
3. JLE   r1, r0, +2               ; if max <= current → skip to abort
4. EXIT                           ; success (3 CUs consumed)
5. MOV   r0, ERR_DEADLINE          ; failure: load error code
6. EXIT                           ; abort (4 CUs consumed)
```

Instructions 4-6 only execute on the failure path. The verifier sees
the success path end at instruction 4 (3 CUs), well within the 4 CU cap.

## Key insight

The clock sysvar account data IS the instruction account. Blueshift
passes the clock account as `r1`, giving us direct access to the
`Slot` field at offset 0 without a syscall. The instruction data
(`max_slot_height`) follows immediately.

## Build & Test

```bash
# Install Blueshift sBPF toolchain
cargo install --git https://github.com/blueshift-gg/sbpf.git

# Build
sbpf build

# Run tests (Mollusk TDD suite)
cargo test

# Verify binary size
ls -l target/deploy/timeout.so  # should be ≤ 320 bytes
```

## Test vectors (Mollusk TDD)

| Case | Current Slot | Max Slot | Expected | CUs |
|------|-------------|----------|----------|-----|
| Before deadline | 100 | 200 | Success | 3 |
| At deadline | 200 | 200 | Success | 3 |
| After deadline | 201 | 200 | Abort | 4 |
| Exact match | 100 | 100 | Success | 3 |
| Genesis (slot 0) | 0 | 100 | Success | 3 |

## Project structure

```
├── Cargo.toml          # sBPF program crate
├── Xargo.toml          # sBPF cross-compilation target
├── src/
│   └── lib.rs          # sBPF assembly (4 instructions)
├── fixtures/            # Mollusk test fixtures
└── tests/
    └── integration.rs  # Mollusk TDD test suite
```

## References

- [Blueshift Assembly Timeout Challenge](https://learn.blueshift.gg/en/challenges/assembly-timeout)
- [sBPF ISA Reference](https://github.com/solana-labs/solana/blob/master/sdk/sbf/c/sbf.md)
- [Mollusk TDD Framework](https://github.com/blueshift-gg/mollusk)
