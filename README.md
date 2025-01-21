# tmux-rs

An in-progress rust port of tmux. This project should be considered pre-alpha quality and may not necessarily be in a building state.

I don't think or want this project to displace tmux. I love using tmux.
Tmux will also continue development while I'm working on this port,
so the structure of the rust code must match that of the C project.
I tried out using [zellij](https://zellij.dev/) once and gave up when the compilation time was something like 40 minutes on my machine.
For me, this refactor is kinda like gardening (I don't garden, but if I did I'd imagine that's what it's like).
Every minute I spend on this is a minute I don't spend mindlessly playing some video game, so I take it as a net win.

I started this endeavour as a way of getting first hand experience with using C2Rust.
It's simultaneously a great and a terrible tool. I was amazed when I used it that it
was able to produce rust code which compiled to a binary which was effectively equivalent to the original C binary.
Unfortunately, the resulting rust code ... leaves a lot to be desired.
My initial approach was starting with the mess generated by C2Rust and slowly de-duplicating
the section generated from the headers and then manually re-writing file by file.

The problem with this approach is the C2Rust duplicates the definitions from C `.h` header files.
This means every ~200 line C file results in a >1000 line rust file.
Furthermore, the Rust code generated is unsafe (this is okay), and unreadable (unacceptable).
The generated code doesn't retain the original intent of the C code, though it may be equivalent.
Think using constants like `42` instead of `b'*'` when the original C code used `'*'`.
The code may be equivalent, but it renders the resulting Rust code useless without the original C source.
That's not the goal of a port. You want to be able to through away the original code after the port and not have lost any information.

Eventually, I got to a point where I had so much in progress code in a non-building state that I gave up
and had to rethink this approach to porting. I've since started over many times.
My current approach is creating many "micro-crates" each of which correspond to each `.c` translational unit.
For now, this approach requires some manual editing of generated Makefiles. This isn't ideal.
If my understanding of autotools was better, I'd try to seamlessly integrate the building of the rust
crates into the original project structure. For now I have a `build.sh` script in the root which
calls make and cargo.

I think the better approach is to mimic the C functions abi with rust extern ffi definitions.
Then successively convert the ffi bindings into implementations. Using a single crate with modules
so it's easy to handle circular deps. I guess the problem is build system. It's difficult to have
a single file partially implemented. each can be all or nothing


# Steps

1. Pick a C file to port
2. Modify Makefile.am to remove the C file from sources list
3. Re-implement c file in rust
4. Change tmux_h/src/lib.rs to re-export rust definitions instead of using extern "C".

# Progress

Current status: building, but aborting immediately.

# TODO
- dump backtrace on abort
- research extern c-unwind vs c
- run under miri

Next, need to remove todo calls causing crashes.

- [ ] 325 alert
  - [ ] implement TODO's
- [ ] 1097 arguments
- [ ] 108 attributes
- [ ] 277 cfg
- [ ] 809 client
- [ ] 175 cmd-attach-session
- [ ] 107 cmd-bind-key
- [ ] 143 cmd-break-pane
- [ ] 253 cmd-capture-pane
- [ ] 117 cmd-choose-tree
- [ ] 242 cmd-command-prompt
- [ ] 163 cmd-confirm-before
- [ ] 98 cmd-copy-mode
- [ ] 109 cmd-detach-client
- [ ] 502 cmd-display-menu
- [ ] 159 cmd-display-message
- [ ] 312 cmd-display-panes
- [ ] 116 cmd-find-window
- [ ] 1314 cmd-find
- [ ] 190 cmd-if-shell
- [ ] 180 cmd-join-pane
- [ ] 67 cmd-kill-pane
- [X] cmd-kill-server
- [ ] 71 cmd-kill-session
- [ ] 110 cmd-kill-window
- [ ] 81 cmd-list-buffers
- [ ] 102 cmd-list-clients
- [ ] 372 cmd-list-keys
- [ ] 148 cmd-list-panes
- [ ] 90 cmd-list-sessions
- [ ] 130 cmd-list-windows
- [ ] 113 cmd-load-buffer
- [ ] 79 cmd-lock-server
- [ ] 122 cmd-move-window
- [ ] 370 cmd-new-session
- [ ] 159 cmd-new-window
- [ ] 159 cmd-parse
- [ ] 113 cmd-paste-buffer
- [ ] 230 cmd-pipe-pane
- [ ] 899 cmd-queue
- [ ] 335 cmd-refresh-client
- [ ] 81 cmd-rename-session
- [ ] 62 cmd-rename-window
- [ ] 215 cmd-resize-pane
- [ ] 115 cmd-resize-window
- [ ] 98 cmd-respawn-pane
- [ ] 95 cmd-respawn-window
- [ ] 115 cmd-rotate-window
- [ ] 290 cmd-run-shell
- [ ] 120 cmd-save-buffer
- [ ] 149 cmd-select-layout
- [ ] 242 cmd-select-pane
- [ ] 150 cmd-select-wind
- [ ] 237 cmd-send-keys
- [ ] 147 cmd-server-access
- [ ] 137 cmd-set-buffer
- [ ] 119 cmd-set-environment
- [ ] 239 cmd-set-option
- [ ] 143 cmd-show-environment
- [ ] 107 cmd-show-messages
- [ ] 260 cmd-show-options
- [ ] 108 cmd-show-prompt-history
- [ ] 208 cmd-source-file
- [ ] 199 cmd-split-window
- [ ] 148 cmd-swap-pane
- [ ] 94 cmd-swap-window
- [ ] 142 cmd-switch-client
- [ ] 104 cmd-unbind-key
- [ ] 264 cmd-wait-for
- [ ] 874 cmd
- [ ] 1117 colour
- [X] compat
- [ ] 262 control-notify
- [ ] 1117 control
- [ ] 281 environ
- [ ] 859 file
- [ ] 5294 format
- [ ] 1243 format-draw
- [ ] 429 grid-reader
- [ ] 235 grid-view
- [ ] 1535 grid
- [ ] 239 hyperlinks
- [ ] 794 input-keys
- [ ] 3025 input
- [ ] 435 job
- [ ] 692 key-bindings
- [ ] 477 key-string
- [ ] 370 layout-custom
- [ ] 691 layout-set
- [ ] 1120 layout
- [X] log
- [ ] 556 menu
- [ ] 1266 mode-tree
- [ ] 172 names
- [ ] 323 notify
- [ ] 1370 options-table
- [ ] 1204 options
- [ ] 342 paste
- [ ] 818 popup
- [ ] 388 proc
- [ ] 120 regsub
- [ ] 467 resize
- [ ] 868 screen-redraw
- [ ] 2347 screen-write
- [ ] 740 screen
- [ ] 557 server
    - [ ] implement TODO's
- [ ] 186 server-acl
- [ ] 3392 server-client
- [ ] 493 server-fn
- [ ] 759 session
- [ ] 497 spawn
- [ ] 2035 status
- [ ] 383 style
- [ ] 538 tmux.c
- [X] tmux.h
- [ ] tmux-protocol.h
- [ ] 269 tty-acs
- [ ] 510 tty-features
- [ ] 1591 tty-keys
- [ ] 924 tty-term
- [ ] 3186 tty
- [ ] 100 utf8-combined
- [ ] 822 utf8
- [X] window
- [ ] 559 window-buffer
- [ ] 418 window-client
- [ ] 286 window-clock
- [ ] 5786 window-copy
- [ ] 1512 window-customize
- [ ] 1348 window-tree
- [X] xmalloc

# Notes

## Compat

tmux is a *bsd project.
I'm not sure which bsd exactly, but it's clear from reading the source code there's many libc functions
used which don't exist on linux, and are provided by bsd. The tmux project makes use of code in the compat
directory and autotools to shim these functions on OS's which they aren't provided. The first area to port
is this. Many linux distro's provide some of these functions already implemented through a library called
libbsd. I made a libbsd-sys library that provides auto-generated rust bindings to this C library. The surface
area of these functions is quite small and could easily be reimplemented later to remove this dependency.

## queue.h and tree.h

The tmux project makes extensive use of macros in the `compat/queue.h` and `compat/tree.h` headers which
implement an intrusive linked list and intrusive red black tree. For the most part, I've been able
to mirror the implementations at the source level using Rust generics. This is a key area to get right.
The auto-generated expanded C macros generated a mess from this code. This code needs to be hand crafted
properly to make use of rust generics which is abi compatible with the original C code. Maybe in the future
it would make sense to instead make use of a crate which provides the same functionality such as [intrusive_collections](https://docs.rs/intrusive-collections/latest/intrusive_collections/).

## C pointer field access operator `->`

Once annoyance of porting C code which makes heavy use of pointers is having to convert uses of the `->` operator.
Rust has no such operator and pointers don't implement deref, so they must be translated to something like `(*w).field`.

For a bit, I thought I could implement by own smart pointer type which wrapped a `*mut T` or `NonNull` and also
implemented DerefMut. Unfortunately doing this requires that you can create a `&mut T` which would likely invoke
undefined behaviour in this context.

# References

- [tmux](https://github.com/tmux/tmux)
- [C2Rust](https://github.com/immunant/c2rust)
- [rust-bindgen](https://rust-lang.github.io/rust-bindgen/)
- [Compiling C to Safe Rust, Formalized](https://arxiv.org/abs/2412.15042)
- [Porting C to Rust for a Fast and Safe AV1 Media Decoder](https://www.memorysafety.org/blog/porting-c-to-rust-for-av1/)
- [Fish 4.0: The Fish Of Theseus](https://fishshell.com/blog/rustport/)
- [Immunant's C2Rust tmux](https://github.com/immunant/tmux-rs)
