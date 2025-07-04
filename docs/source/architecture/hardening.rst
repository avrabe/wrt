======================
Arm Hardening Features
======================


For comprehensive Control Flow Integrity (CFI) implementation details including BTI, RISC-V CFI, and software CFI fallback, see :doc:`cfi_hardening`.

Enabling Hardening
------------------


.. code-block:: bash

   cargo build --features helper-mode,arm-hardening,platform-<your-platform>

Features
--------

The `arm-hardening` flag enables:

1.  **Pointer Authentication (PAC) & Branch Target Identification (BTI)**:
    *   Adds compiler flag `-mbranch-protection=standard`.
    *   Protects against ROP/JOP attacks by signing/authenticating function pointers and ensuring branches land at valid BTI instructions.
    *   Requires compatible hardware (e.g., Armv8.3-A and later) and toolchain support.

2.  **Memory Tagging Extension (MTE)**:
    *   Adds compiler flag `-fsanitize=memtag`.
    *   Requires the underlying platform's `PageAllocator` to map memory with MTE-enabled flags (e.g., `PROT_MTE` on Linux, equivalent on macOS/QNX when available).
    *   Assigns tags to memory allocations and pointers. Hardware checks tags on access, trapping on mismatch to detect spatial and temporal memory errors (use-after-free, buffer overflows).
    *   Supported on compatible hardware (e.g., Armv8.5-A and later like Apple Silicon, Neoverse V1/N2) and OS (Linux >= 5.14, recent macOS/QNX).

`no-std` Compatibility
--------------------

The `arm-hardening` feature preserves `no-std` compatibility. On bare-metal or platforms lacking OS support for MTE (like Zephyr initially), MTE mapping is disabled, but PAC/BTI can still be enabled if the toolchain and startup code support it.

Verification
------------

*   **PAC/BTI**: Check for appropriate flags in `readelf -A <binary>` output.
*   **MTE**: Run unit tests under an MTE-enabled environment (e.g., Linux with `CONFIG_ARM64_MTE=y` and MTE boot args) or using the LLVM MTE sanitizer included with `-fsanitize=memtag`.

References
----------

*   Linux Memory Tagging Extension: Kernel docs `arm64/memory-tagging-extension` (docs.kernel.org)
*   Arm `-mbranch-protection` flags: Armclang manual (Arm Developer)
*   macOS pointer authentication (arm64e): Community RE summary (GitHub)
*   QNX memory protection: `mprotect()` manual page (qnx.com) 