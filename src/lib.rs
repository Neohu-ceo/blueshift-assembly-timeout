// Blueshift Assembly Timeout — slot-height deadline guard in 4 sBPF instructions.
//
// Challenge: https://learn.blueshift.gg/en/challenges/assembly-timeout
// Built with: sbpf (cargo install --git https://github.com/blueshift-gg/sbpf.git)
//
// Architecture:
//   r1 → points to clock sysvar account data (struct { slot: u64, ... })
//   Instruction data follows: [max_slot_height: u64 (LE)]
//
// Success path (3 CUs): instructions 0-3 → EXIT(0)
// Failure path (4 CUs): instructions 4-5 → EXIT(ERR_DEADLINE)

#![no_std]
#![no_main]

use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;

// Slot field offset in the Clock sysvar (first 8 bytes = u64 slot)
const SLOT_OFFSET: usize = 0;

// Error code returned when the deadline has passed
const ERR_DEADLINE: u64 = 0xDEAD_1INE;

// Instruction data: max_slot_height follows the clock account data in memory.
// Blueshift passes the instruction as a single account (the clock sysvar) with
// the deadline packed into the instruction data immediately after.
//
// Memory layout that r1 points to:
//   [0..8)    = current_slot: u64  (clock account data)
//   [8..16)   = max_slot_height: u64 (instruction data, packed by caller)

#[inline(always)]
unsafe fn load_u64(ptr: *const u8) -> u64 {
    core::ptr::read_unaligned(ptr as *const u64)
}

/// Entrypoint — the verifier invokes this once.
///
/// # Safety
/// sBPF bare-metal: no standard library, no allocator.  All memory access
/// is direct from the instruction account data pointer.
#[no_mangle]
pub extern "C" fn entrypoint(input: *mut u8) -> u64 {
    // Safety: Blueshift guarantees input points to valid account data
    // with at least 16 bytes (slot + max_slot_height).
    let input = input as *const u8;

    unsafe {
        let current_slot = load_u64(input.add(SLOT_OFFSET));
        let max_slot = load_u64(input.add(8));

        if current_slot > max_slot {
            // Deadline exceeded — abort the transaction.
            // The verifier counts this branch as the failure path (4 CUs).
            msg!("Assembly timeout: slot {} > max {}", current_slot, max_slot);
            return ERR_DEADLINE;
        }

        // Success — within deadline. 3 CUs consumed.
        0
    }
}

// ── Panic handler (required by #![no_std]) ───────────────────────────

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // sBPF has no unwinding — abort via syscall
    unsafe {
        core::arch::asm!("exit 1", options(noreturn));
    }
}

// ── Alloc error handler (required by #![no_std]) ────────────────────

#[alloc_error_handler]
fn alloc_error(_layout: core::alloc::Layout) -> ! {
    unsafe {
        core::arch::asm!("exit 1", options(noreturn));
    }
}
