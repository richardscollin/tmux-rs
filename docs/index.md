---
layout: post
title: Introducing tmux-rs
author: Collin Richards
date: 2025-07-03
---
For the 6 months or so I've been quietly porting tmux from C to Rust. I've recently reached a big milestone: the code base is now 100% (unsafe) Rust.
I'd like to share the process of porting the original codebase from ~67,000 lines of C code to ~81,000 lines of Rust (excluding comments and empty lines).
You might be asking: why did you rewrite tmux in Rust? And yeah, I don't really have a good reason. It's a hobby project. Like gardening, but with more segfaults.

- [Starting with C2Rust](#starting-with-c2rust)
- [Build process](#build-process)
- [Interesting bugs](#interesting-bugs)
  - [Bug 1](#bug-1)
  - [Bug 2](#bug-2)
- [C Patterns in Rust](#c-patterns-in-rust)
  - [Raw pointers](#raw-pointers)
  - [Considering Goto](#considering-goto)
  - [Intrusive Macros](#intrusive-macros)
  - [Yacc shaving](#yacc-shaving)
- [Development process](#development-process)
  - [Vim](#vim)
  - [AI Tools](#ai-tools)
- [Conclusion](#conclusion)

## Starting with C2Rust

I started this project as a way of trying out [C2Rust](https://github.com/immunant/c2rust), a C to Rust transpiler. The tool was a little tricky to set up, but once it was running the generated output was a successful port of the tmux codebase in Rust.

Despite the generated code working, it was basically unmaintainable and 3x larger than the original C. You wouldn't want to touch it with a 10 foot pole. Here's an example of the output:

```c
// original C code
int colour_palette_get(struct colour_palette *p, int c) {
  if (p == NULL)
    return (-1);

  if (c >= 90 && c <= 97)
    c = 8 + c - 90;
  else if (c & y)
    c &= ~COLOUR_FLAG_256;
  else if (c >= 8)
    return (-1);

  if (p->palette != NULL && p->palette[c] != -1)
    return (p->palette[c]);
  if (p->default_palette != NULL && p->default_palette[c] != -1)
    return (p->default_palette[c]);
  return (-1);
}
```

```rust
// generated Rust code
#[no_mangle]
pub unsafe extern "C" fn colour_palette_get(
    mut p: *mut colour_palette,
    mut c: libc::c_int,
) -> libc::c_int {
    if p.is_null() {
        return -(1 as libc::c_int);
    }
    if c >= 90 as libc::c_int && c <= 97 as libc::c_int {
        c = 8 as libc::c_int + c - 90 as libc::c_int;
    } else if c & 0x1000000 as libc::c_int != 0 {
        c &= !(0x1000000 as libc::c_int);
    } else if c >= 8 as libc::c_int {
        return -(1 as libc::c_int)
    }
    if !((*p).palette).is_null()
        && *((*p).palette).offset(c as isize) != -(1 as libc::c_int)
    {
        return *((*p).palette).offset(c as isize);
    }
    if !((*p).default_palette).is_null()
        && *((*p).default_palette).offset(c as isize) != -(1 as libc::c_int)
    {
        return *((*p).default_palette).offset(c as isize);
    }
    return -(1 as libc::c_int);
}
```

This snippet isn't that bad, but things can get a lot worse. My main concern was losing information from named constants like `COLOUR_FLAG_256` which is translated to `0x1000000`. There are also a lot of casts to `libc::c_int` polluting the code as well. I suspect this is to handle [C's integer promotion rules](https://stackoverflow.com/a/46073296). Most of them are completely unnecessary when doing operations on literals in Rust.

I spent quite a lot of time manually refactoring the shitty Rust code to less shitty Rust code, but I kept finding myself having to look at the original C code to understand the program's intent. After manually refactoring many files this way I gave up on this approach. I threw away all of the C2Rust output and decided I would translate all of the files into Rust manually from C.

> Despite not using C2Rust for this project I still think it's a great tool. It was very important for me to actually be able to compile and run the project from the start. It made me realize this endeavour was achievable. I've even integrated it as part of one of my [other side projects](https://crates.io/crates/include).

## Build process

```
┌─────────────┐    ┌────────────┐     ┌──────────────┐    ┌──────────┐         ┌───────┐    
│ Makefile.am │───►│ autogen.sh ├────►│ configure.sh │───►│ Makefile │         │ cargo │    
└─────────────┘    └────────────┘     └──────────────┘    └──────────┘         └───┬───┘    
                                                                                   │        
                                                                                   │        
                                ┌──────┐       ┌──────┐                            │        
                           ┌───►│tmux.c├──────►│tmux.o├───────┐                    │        
               ┌──────┐    │    └──────┘       └──────┘       │                    │        
               │tmux.h├────┤                                  │                    │        
               └──────┘    │  ┌────────┐     ┌────────┐       │                    │        
                           ├─►│window.c├────►│window.o├───────┤                    │        
              ┌────────┐   │  └────────┘     └────────┘       │                    │        
              │compat.h├───┤                                  │                    │        
              └────────┘   │    ┌──────┐       ┌──────┐       │                    │        
                           └───►│pane.c├──────►│pane.o├───────┤                    ▼        
                                └──────┘       └──────┘       │             ┌──────────────┐
                                          ┌───────────┐       │    ┌────┐   │              │
                                          │           │       ├───►│tmux│◄──┤ libtmux_rs.a │
                                          │ libc.so.6 ├───────┤    └────┘   │              │
                                          │           │       │             └──────────────┘
                                          └───────────┘       │                             
                                      ┌───────────────┐       │                             
                                      │               │       │                             
                                      │ libtinfo.so.6 ├───────┘                             
                                      │               │                                     
                                      └───────────────┘                                     
```

The most important part of this rewrite was first developing a solid understanding of how the project was built. For tmux this is `autotools`. I figured out where to add/remove files in `autogen.sh` and how to modify the generated `Makefile` to link in a static library created by my rust crate using the `crate-type = "staticlib"` option.

This did mean my build process wasn't as simple as just running `cargo build`. I wrote a small `build.sh` script which would invoke cargo, then run `make`. This worked for a while, but any time I completed translating a file I had to reconfigure and modify the `Makefile`.

Early on I tried to break things up into mini-crates. It ends up being easier to put everything in the same crate for two reasons: 1. Crates can't have circular dependencies and 2. you can run into linking issues when linking multiple Rust libraries into the same binary.

At first, I would translate one file at a time, with no way to validate the changes halfway through each file. After translating a large file and getting stuck debugging, I changed my development process to translate only one function at a time, with a quick `build.sh run` in between to make sure everything worked. This did mean adding extra headers in the C code for functions which were originally static. The new process looked like this:

- copy the header of the C function
- comment out the C function body

```c
int colour_palette_get(struct colour_palette *p, int c);
// int colour_palette_get(struct colour_palette *p, int c) {
// ...
//
```

- implement the function in Rust

The C code would link against the Rust implementation as long as the function had
the `#[unsafe(no_mangle)]` attribute `extern "C"` annotation and importantly the correct signature.

After translating about half of the C files I started thinking the current build process was a bit silly. Most of the code was now in Rust. Instead of building a C binary and linking in a Rust library I should be building a Rust binary and linking in a C library. Well that's exactly what you can do using the `cc` crate.

I set up a `build.rs` like so:

```rust
// simplified version of tmux-rs/build.rs
fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rustc-link-lib=bsd");
    println!("cargo::rustc-link-lib=tinfo");
    println!("cargo::rustc-link-lib=event_core");
    println!("cargo::rustc-link-lib=m");
    println!("cargo::rustc-link-lib=resolv");

    let mut builder = &mut cc::Build::new();

    static FILES: &[&str] = &[
        "osdep-linux.c",
        "cmd-new-session.c",
        "cmd-queue.c",
        // ...
        "window-customize.c",
        "window-tree.c",
    ];
    for f in FILES {
        builder = builder.file(std::path::PathBuf::from("..").join(f))
    }

    builder.compile("foo");
}

```

## Interesting Bugs

I introduced many bugs while translating the code. I'd like to share the process of discovering and fixing a couple.

### Bug 1

The program started segfaulting after translating a trivial function. The source and translation are below:

```c
void* get_addr(client* c) {
  return c->bar;
}
```

```rust
unsafe extern "C" fn get_addr(c: *mut client) -> *mut c_void {
  unsafe {
    (*c).bar
  }
}
```

After running in the debugger the error was something like: `Invalid read at address 0x2764`.

I walked through the code again. Inside of the Rust function `(*c).bar`
has a valid address, like `0x60302764`, but out the function, the value received from the calling C code
was `0x2764`. Do you know the problem yet? Need another hint? If I looked more closely at the C compilation
warnings I would have seen:

```
warning: implicit declaration of function ‘get_addr’ [-Wimplicit-function-declaration]
```

That's right, the C code was using the implicit declaration which is:

```c
int get_addr();
```

That explains why the value was incorrect! The C compiler was thinking a 4 byte int was returned not an 8 byte pointer.
So the top 4 bytes were being truncated or ignored. The fix was as simple as adding the correct prototype to the C code
and the compiler would generate the correct code.

### Bug 2

Again I noticed this bug after translating a trivial function which shouldn't have caused any issues. It was something like this:

```c
void set_value(client* c) {
  c->foo = 5;
}
```

```rust
unsafe extern "C" fn set_value(c: *mut client) {
  unsafe {
    (*c).foo = 5;
  }
}
```

I was shocked that after translating this simple function the program started segfaulting. Inspecting it in the debugger showed that the segfault in the Rust code was happening on that line, which should be identical to the C. In the debugger I noticed that the address was slightly different in the C from the Rust, maybe that's just do address randomization.

So what's the issue? Well it just so happens that when I manually translated the type declaration of the client struct I missed an `*` on one of the types. This type was just above the data field. Meaning the C and Rust code had different views of the type after that mismatched field.

For example the C struct looked like:

```c
struct client {
  int bar;
  int *baz;
  int foo;
}
```

And the Rust looked like:

```rust
struct client {
  bar: i32,
  baz: i32,
  foo: i32,
}
```

Nothing in the Rust touched `baz` yet, so there were no compiler errors, but the data would be interpreted and accessed incorrectly. The fix this time was as easy as correcting the incorrect types in the Rust code.

## C Patterns in Rust

### Raw pointers

Rust has two reference types: `&T`: a shared reference or `&mut T`: an exclusive (or mutable) reference. A Rust reference is a pointer with several other invariants. One of the invariants is that a Rust reference can never be null and the value pointed to must be fully initialized and valid.

The natural mapping of pointers in a C program would be a reference in Rust, either exclusive or shared depending if it's modified in the code. The problem is, often times some of the invariants required by references in Rust cannot always be upheld if we do a straight one-to-one mapping of the source from C to Rust. That means we can't use Rust references in our port yet. We have to use another type, raw pointers: `*mut T` and `*const T`. Semantically raw pointers are the same as C pointers, but because you don't really use them outside of unsafe Rust they are extremely unergonomic to use.

### Considering Goto

C has `goto`. `goto` gets a bad wrap, but actually it's usage in the tmux codebase is quite tame, and only one or two of the usages actually cause implementation difficulties.

The c2rust transpiler uses an algorithm to emulate goto logic. A good video describing a similar algorithm can be found in this [video](https://www.youtube.com/watch?v=qAeEWKr9wfU). However most cases don't actually require using this algorithm and can instead use a much simpler method.

- Forward jumps can be implemented using a labeled block with a break statement:

```rust
fn foo() {
  'error: {
    println!("hello");

    if random() % 2 == 0 {
      break 'error; // same as goto error in C
    }

    println!("world");
    return;
  } // 'error:
  println!("error");

}
```

- Backward jumps can be implemented using a labeled loop with continue:

```rust
fn bar() {
  'again: loop {
    println!("hello");

    if random() % 2 == 0 {
      continue 'again; // same as goto again in C
    }

    println!("world");
    return;
  }
}
```

These are the most common types of usages of goto in the tmux codebase. Only a handful of more complex goto usage required me getting out a pencil and paper to trace out how to map the control flow (see `window_copy_search_marks` in the codebase if you're interested).

### Intrusive Macros

Tmux makes extensive use of two data structures defined using macros: an intrusion red black tree and linked list. An intrusive data structure is one where pieces of the data structure live within your struct. This is different from how most container data structures are implemented today where the container holds the unmodified struct and doesn't require support from the struct to hold data for the collection.

I went through many iterations of implementing a good Rust interface mimicking the C code. I ended up with this:

```c
// cmd-kill-session.c
RB_FOREACH(wl, winlinks, &s->windows) {
  wl->window->flags &= ~WINDOW_ALERTFLAGS;
  wl->flags &= ~WINLINK_ALERTFLAGS;
}
```

```rust
// cmd_kill_session.rs
for wl in rb_foreach(&raw mut (*s).windows).map(NonNull::as_ptr) {
    (*(*wl).window).flags &= !WINDOW_ALERTFLAGS;
    (*wl).flags &= !WINLINK_ALERTFLAGS;
}
```

The code would actually be cleaner if I didn't return a `NonNull<T>` from the iterator. I implemented my own trait in order to mimic this interface. One
of the challenges of this some instances can live in different containers at the same time. This is problematic because a trait can only be implemented once for a given type. The solution was making the trait generic so that it's not a single trait but multiple depending on the generic parameter. I used a dummy unit type when I need to distinguish which trait to use in the code. Here's the ugly code that enables the nice interfaces which closely resemble the C:


```rust
pub trait GetEntry<T, D = ()> {
    unsafe fn entry_mut(this: *mut Self) -> *mut rb_entry<T>;
    unsafe fn entry(this: *const Self) -> *const rb_entry<T>;
    unsafe fn cmp(this: *const Self, other: *const Self) -> std::cmp::Ordering;
}

pub unsafe fn rb_foreach<T, D>(head: *mut rb_head<T>) -> RbForwardIterator<T, D>
where
    T: GetEntry<T, D>,
{
    RbForwardIterator {
        curr: NonNull::new(unsafe { rb_min(head) }),
        _phantom: std::marker::PhantomData,
    }
}
pub struct RbForwardIterator<T, D> {
    curr: Option<NonNull<T>>,
    _phantom: std::marker::PhantomData<D>,
}

impl<T, D> Iterator for RbForwardIterator<T, D>
where
    T: GetEntry<T, D>,
{
    type Item = NonNull<T>;
    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.curr?.as_ptr();
        std::mem::replace(&mut self.curr, NonNull::new(unsafe { rb_next(curr) }))
    }
}

```

### Yacc shaving

Tmux uses `yacc` to implement a custom parser for it's configuration language. I was aware of `lex` and `yacc` before, but had never used them. The last step to converting the project from C to Rust was figuring out how to reimplement the parser in `cmd-parse.y` from `yacc` to Rust. After completing this I'd be able to completely shed the `cc` crate and streamline the build process.

After one or two failed attempts with different crates I settled on using the `lalrpop` crate to implement the parser. The structure of lalrpop code closely matches `yacc` which allowed me to do a one-to-one reimplementation like the rest of the project.

The original yacc parser looks like this:

```c
lines		: /* empty */
		| statements
		{
			struct cmd_parse_state	*ps = &parse_state;

			ps->commands = $1;
		}

statements	: statement '\n'
		{
			$$ = $1;
		}
		| statements statement '\n'
		{
			$$ = $1;
			TAILQ_CONCAT($$, $2, entry);
			free($2);
		}
```

It's a grammar with a series of actions to perform when the rules are matched.
The equivalent section of the grammar translates to the following `lalrpop` snippet.

```rust
grammar(ps: NonNull<cmd_parse_state>);

pub Lines: () = {
    => (),
    <s:Statements> => unsafe {
      (*ps.as_ptr()).commands = s.as_ptr();
    }
};

pub Statements: NonNull<cmd_parse_commands> = {
    <s:Statement> "\n" => s,
    <arg1:Statements> <arg2:Statement> "\n" => unsafe {
      let mut value = arg1;
      tailq_concat(value.as_ptr(), arg2.as_ptr());
      free_(arg2.as_ptr());
      value
    }
};
```

> `lalrpop` has a few bugs, for example it can't handle raw pointers properly (the * seems to throw off the parser), that's fine I just ended up using `NonNull<T>` in all the places instead.

After reimplementing the grammar, I also had to implement an adapter to interface lalrpop with the custom lexer. The lexer was the same from the original codebase, just wrapped in a Rust iterator. I was amazed that once the lexer was hooked up to the parser it just seemed to work. This last step enabled me to get rid of all of the remaining C code and headers.

## Development process

### Vim

Throughout working on this project I used many different text editors and ides. My typical workflow used neovim while heavily relying on custom macros to speed up the translation process. For example, I made vim macros for things like converting:

- `ptr == NULL` to `ptr.is_null()`
- `ptr->field` to `(*ptr).field`

Most of these mechanical changes are very easy to make, but are hard to do all at once with a find and replace. This means doing it by hand thousands of times.

### AI Tools

I did start trying out Cursor towards the end of the development process. I ended up stopping using it though because I felt like it didn't actually increase my speed. It only saved me from finger pain. That's because when using cursor to translate the code it would still occasionally insert bugs, just like me. So, I spent as much time reviewing the generated code as it would have taken me to write it myself. The only thing it saved was my hands. Doing this large amount of refactoring is really hard on your fingers. 

So, even though I quit using cursor, my feeling is that I'd still reach for it if my hands are really physically hurting, and I need to keep working. Usually once I reach the point where I've got blisters on my fingers I think it's better to just take a break. Given the pace at how fast the AI tooling is developing I wouldn't be surprised if this project could be accomplished in significantly less time using a different approach.

## Conclusion

Even though the code is now 100%, I'm not sure I've accomplished my main goal yet. My hand translated code isn't that much better than the output from C2Rust. It's also not very difficult to get it to crash and I am aware of many bugs. The next goal is to convert the codebase to safe Rust.

Despite all of this, I'm releasing version 0.0.1 to share with other fans of Rust and `tmux`. If this project interests you, you can connect with me through [Github Discussions](https://github.com/richardscollin/tmux-rs/discussions). See the installation instructions in the [README](https://github.com/richardscollin/tmux-rs).

