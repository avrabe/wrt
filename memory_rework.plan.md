Below is a design-background & requirements update that you can drop straight into the docs/ tree (for instance under docs/source/architecture/ or docs/source/rework/).
Everything is phrased so that an implementation-agent can start coding, and the documentation team can see exactly where new content has to go.

1 Background & motivation — why are we doing this?
wrt today is a purely-in-process WebAssembly interpreter written in safe Rust.
It allocates linear memory with Vec<u8>, uses host threading in ad-hoc ways, and assumes a POSIX-like OS with no memory-tagging or pointer-authentication support.

The new ahead-of-time (AOT) translator will emit raw AArch64 machine code and link against a tiny "helper" library that implements only those Wasm operations that cannot be lowered 1-to-1 (memory.copy, memory.grow, atomic.wait, …).
That helper must therefore:

run on very different environments (macOS, Linux, QNX Neutrino, Zephyr RTOS, or bare-metal),

expose a stable C ABI so the translator can simply BL wrt_memory_copy,

optionally use modern Arm hardening features—PAC/BTI for control-flow integrity and MTE for memory-safety.

MTE requires pages mapped with the PROT_MTE flag on Linux ≥ 5.14 
docs.kernel.org
 and equivalent flags on macOS/QNX when available.

The rework therefore introduces a platform abstraction layer and splits the build into two feature profiles:

engine = current interpreter (default)
helper-mode = stripped-down C-ABI runtime for AOT output

2 High-level technical changes
Area	Before	After
Memory backend	Vec<u8> inside Rust	LinearMemory<P: PageAllocator>; back-ends for macOS, Linux, QNX, Zephyr, bare-metal
Syscalls / wait-notify	OS-direct, Linux-only	FutexLike trait with per-OS impl (futex, __ulock, SyncCondvar, k_futex, spin-loop)
Hardening	none	Opt-in cargo feature arm-hardening adds PAC/BTI (-mbranch-protection=standard) 
Arm Developer
Arm Developer
 and MTE (PROT_MTE / fsanitize=memtag)
Public surface	interpreter API only	adds wrt-helper.h (C prototypes) + libwrt_helper.{a,so}
Build targets	Linux host only	arm64 macOS → Linux → QNX → bare-metal (Zephyr in parallel)

3 Where to put the new documentation

The architecture documentation has been restructured into the `docs/source/architecture/` directory.

Key files related to this rework:

```
docs/
└── source/
    ├── architecture/
    │   ├── index.rst           (Overview, includes links to all sections)
    │   ├── memory_model.rst    (Describes memory; Needs update for new model & diagrams)
    │   ├── platform_layer.rst  (NEW - Platform Abstraction Layer design)
    │   └── hardening.rst       (NEW - Arm Hardening features)
    └── requirements/
        ├── requirements.rst          (Contains :req:`REQ_PLATFORM_001`, :req:`REQ_HELPER_ABI_001`)
        └── safety_requirements.rst (Contains :req:`REQ_MEMORY_001`, :req:`REQ_SECURITY_001`)
```

(Ensure all new/updated pages include a `References` subsection as needed, using links from section 6 below.)

4 Relevant Requirements

The following requirements defined in the main documentation tree capture the goals of this rework:

*   :req:`REQ_PLATFORM_001` (Platform Abstraction Layer)
*   :req:`REQ_MEMORY_001` (Platform-Managed Linear Memory)
*   :req:`REQ_HELPER_ABI_001` (Helper Runtime C ABI Exports)
*   :req:`REQ_SECURITY_001` (Optional Arm Hardening Features)

(The detailed objectives, background, and acceptance criteria are defined within these requirement entries.)

5 Implementation sequence (first macOS, then Linux, QNX, bare-metal)
Phase	Target	Key tasks
1	macOS arm64	Platform back-end: `mmap(PROT_READ
2	Linux aarch64	Use mmap + PROT_MTE, futex syscalls; integrate fsanitize=memtag.
3	QNX 7.1 (aarch64)	mmap + MAP_LAZY; wait/notify via SyncCondvar APIs; optional PROT_MTE once toolchain patch lands.
4	Bare-metal (EL1-N)	Bump allocator for 64 KiB pages; simple WFE/SEVL spin futex; optional PAC/BTI in crt0.S. Zephyr back-end can reuse most of this but swap in k_futex_* primitives 
docs.zephyrproject.org
.

Each phase must:

add the backend module in wrt-platform,

add a CI job that cross-compiles wrt-helper,

update docs to list the new target.

6 Key external references
Topic	Reference
Linux Memory Tagging Extension	Kernel docs arm64/memory-tagging-extension 
docs.kernel.org
Arm -mbranch-protection flags	Armclang manual 
Arm Developer
Arm Developer
macOS pointer authentication	Community reverse-engineered summary (arm64e) 
GitHub
Zephyr futex API	k_futex_wait/k_futex_wake docs 
docs.zephyrproject.org
QNX memory protection	mprotect() manual page 
qnx.com

(Add these links in a References subsection at the end of each new doc page.)

Hand-off — what the agent does next

(Steps below assume the documentation restructuring and requirement updates described above are complete).


2.  Implement the macOS platform back-end (`PageAllocator` / `FutexLike` traits).
3.  Refactor `wrt-types` (`LinearMemory`) to use the PAL; adjust all callers.
4.  Refine `docs/source/architecture/memory_model.rst`:
    *   Fully describe the new `LinearMemory`/`PageAllocator` interaction.
    *   Embed the `memory.grow` sequence diagram.
5.  Update `docs/source/architecture/index.rst`:
    *   Embed the updated System Component diagram.
6.  Run the existing unit tests under `cargo test --features helper-mode,platform-macos`.
7.  Open a pull request tagged "Phase 1 macOS backend".

After merge, the same pattern repeats for Linux, QNX, and bare-metal phases, each time implementing the platform backend, expanding the documentation (platform details, CI matrix), and adding CI jobs.

That should give the implementation agent a clear line-of-sight from first commit to full multi-platform helper runtime.