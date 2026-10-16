# Actual padded cursor CPU consumer boundary

These four source files match the selected
[actual native x86 consumer RED→GREEN](../../../docs/native-render-import.md#actual-padded-cursor-consumer-red).
The single case is separate from the five C++/Rust FFI groups, no-attach identity
case and seven Source/rendering-stall cases. No private controller, stager, raw
receipt, process log, compositor binary or SDK archive is published here.

`real-wayland-client.consumer.c` runs with app ID, original absolute
CLOCK_BOOTTIME cutoff and `--consumer-profile`. It creates a real public virtual
pointer bound to its actual seat/output. The owned driver must focus the mapped
client with current Lua dispatch, verify actual window/output geometry, and send:

- `P`: genuine absolute zero→center motion/frame requests plus a real display
  sync. Require actual POINTER capability, pointer-enter event and server serial.
- `A`: assign an ordinary original-pattern XRGB SHM buffer and request a frame
  callback. Require actual commit/release, flush, display sync and frame callback.
- `C`: use that actual pointer-enter serial to set a real32×32 ARGB cursor with
  stride144, unique visible row/column pixels and16 padding bytes per row `0xa5`.
  Require actual cursor commit/release, flush and display sync.
- `Q`: disconnect normally. Retain actual parent wait0, dead pidfd, absent process
  and PIPE EOF before the original owner cutoffs, including failure cleanup.

Compile the client against libwayland-client and generated XDG-shell and
[virtual-pointer client code](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/protocols/wlr-virtual-pointer-unstable-v1.xml).
It contains no session-lock code or lock-header dependency. It does not fabricate
a role or pointer serial or substitute a second input route if setup fails.

`shm-consumer-witness.cpp` is a test-only plugin for the exact rebuilt compositor
SDK, including the actual Rust C ABI and `pointer/CursorImage.hpp`; link Cairo.
The command prefix is `shm_consumer_witness`. `retain window|cursor PID DEADLINE
SURFACE_ID` verifies the actual PID/UID1000/resource and, for cursor, the current
PointerManager-selected cursor role. It holds one Rust clone per kind under a
common at-most30s deadline. The full snapshot pattern/padding and cursor cache
must agree before it invokes the linked production `Pointer::cursorCpuInputSurface`.
It paints that returned Cairo input into an owned CPU image and compares every
visible pixel. A valid mismatch returns `ok:true` with bounded mismatch details,
so it remains a semantic result rather than a setup failure. `verify-live` repeats
that observation; `verify-retained` checks the clone after actual client exit
without querying a server surface. `close` must release two handles and hold zero.

`consumer_oracle.py` always requires original output rows after all positive
controls and owned closure. Exit2 means setup/closure unqualified; exit1 means an
actual qualified row mismatch; exit0 means the same property passes.
`consumer_epoch.py` joins this result to the actual owner and expected epoch:
outer RED success requires actual controller1 plus that mismatch and complete
closure. These files do not provide a complete one-command VM reproduction.

The actual baseline retains all4,608 cursor bytes and cache content, but its
production helper uses input stride128 and paints992 wrong pixels out of1,024,
including original padding at `(0,1)`. A repair using supplied stride is authored
after this RED and passes its matching native rebuild/five FFI groups. The same
case then passes actual GREEN with all five actor/controller/oracle/join hashes
unchanged: two live pre-client-exit Cairo paints each check1,024 pixels with zero
mismatches, source/input stride144 and CPU destination stride128. After actual
`Q`/client wait0, both Rust clones separately verify raw original pixels/padding;
this post-exit check does not call Cairo or query a live server surface. Explicit
close releases two handles and leaves zero; full original owned closure passes.
Both earlier focus/motion setup failures remain setup failures.

This exercises the real linked Cairo CPU consumer normally used in preparation
of a hardware cursor buffer. The full hardware-cursor branch and software-cursor
display path are not qualified. Assigned snapshot bytes and a frame callback do
not establish GPU upload, output readback or presentation; retained clones after
client exit do not establish server resource retirement. Source/Observer are not
selected. Original cold full-owner/Sounds/PCM, ARM/Google-kernel and phone gates
remain open.
