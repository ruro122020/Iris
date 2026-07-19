---
title: Result, Errors Are Ordinary Values
date: 07-07-2026
description: Why Rust has no exceptions. `Result<T, E>` makes failure an ordinary value the caller must handle, and four tools (`match`, `?`, `unwrap`, `expect`) answer it.
draft: false
---

# `Result`: Errors Are Ordinary Values

🔑 Core Concept
Format: the concept, the mental model, the check questions, and the answers worth remembering.

**Introduced while writing:** `src/main.rs` `main` signature `Result<(), std::io::Error>`

### The code that introduced it

Iris starts a web server, and starting a server can fail: the port might already be in use. So `main`
is declared to return a `Result`, and its last line is `Ok(())`:

```rust
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // ... build the router, bind a port, serve ...

    Ok(())
}
```

Two things in that signature deserve an explanation: why a function announces its failures in its
return type at all, and what `Ok(())` is doing on the last line.

### The concept

> **🔑 Core Concept: `Result`, errors as ordinary values**
>
> Rust has no exceptions. A function that can fail returns `Result<T, E>`: an **enum** (a type that is exactly one of several listed variants) with two variants, `Ok(T)` carrying the success value, or `Err(E)` carrying the error.
>
> Failure is part of the function's *type signature*, checked by the compiler. You cannot forget to handle it the way you can forget a `try`/`catch`, because the success value is locked inside the `Ok` wrapper, and the only way to reach it is to deal with both variants.
>
> Our `main` returns `Result<(), std::io::Error>`. The `()` is the **unit type**, Rust's "nothing meaningful here" value, similar to `void`. So the signature reads as "either succeeds with nothing to report, or fails with an I/O error." When `main` returns a `Result`, the process exit code becomes 0 on `Ok` and nonzero on `Err` (with the error printed), which is exactly the contract shell tools and systemd expect.
>
> That's why the last line is `Ok(())`: "reached the end, succeeded, nothing to hand back." No semicolon and no `return`, because in Rust the final expression of a block *is* the block's value.

The shift from Python/JS: in Python, `open("x.txt")` has a signature that says nothing about
failure; you discover it can throw an error only from docs or from being burned at runtime. In Rust,
`Result<File, io::Error>` says "this can fail" at **compile time, in the signature itself**. Errors
move from a runtime surprise to a compile-time fact.

### The forced question, and the four tools that answer it

The value you want (say a `File`) is **inside** the `Ok(File)`, and the wrapper might be holding an `Err(e)`
instead. Before the compiler lets you touch the inner value, you must account for the `Err`. Trying
to call a `File` method directly on a `Result<File, io::Error>` does not compile. Four ways to
answer "what do you want to do about the `Err` first":

**1. `match`, the foundational one. Handle both variants explicitly.**

```rust
let file = match File::open("config.txt") {
    Ok(f) => f,
    Err(e) => {
        println!("could not open: {e}");
        return;
    }
};
```

`Ok(f)` and `Err(e)` are **patterns**: they destructure the enum (pull the inner value out) *and*
branch on the variant, at once. The compiler checks the match is **exhaustive**; omit the `Err`
arm and it refuses to compile. That exhaustiveness is the enforcement: an error cannot slip through.

**2. `?`, the everyday idiom. Pass the error upward.**

```rust
let file = File::open("config.txt")?;
```

Read `?` as: "if `Ok`, unwrap and continue; if `Err`, return that `Err` out of the current function
right now." Only compiles inside a function whose return type can hold that error (like `main`
returning `Result<(), io::Error>`). This is what you type nearly every time.

**3 and 4. `unwrap` and `expect`, the escape hatches. Bet it won't fail; panic if it does.**

```rust
let file = File::open("config.txt").unwrap();            // panic if Err
let file = File::open("config.txt").expect("no config"); // panic with a message
```

A **panic** crashes the current thread, unwinding the stack. Blunt: it converts a recoverable error
into a crash. Acceptable only in tests,
throwaway code, or a genuinely impossible case (and even then, document why).

Mental model: `match` = handle explicitly, `?` = pass it up, `unwrap`/`expect` = bet it won't happen, panic if it does.

### Check questions (and the answers that matter)

1. **Reading `Result<File, io::Error>` alone, before running anything, what do you know that a
   Python caller of `open()` cannot know from the signature?**
   That the call can fail, and with what error type, at compile time. Python hides failure from the
   signature; you learn it from docs or at runtime.

2. **The `File` is inside `Ok(File)`. What does the compiler force at every call site, and when?**
   You must account for the `Err` case before reaching the inner value, enforced at compile time.
   Calling a `File` method straight on a `Result` does not compile. The one case you're never
   allowed to silently ignore is the `Err`.

3. **`TcpListener::bind("127.0.0.1:3000").await?` and the port is taken. Which behavior happens,
   and where does the error end up?**
   The `?` sees the `Err`, returns it out of `main`. `main`'s signature allows `Err(io::Error)`, so
   the runtime's blocking wrapper receives it, prints it, and the process exits nonzero. The error
   is a value handed back up the call chain, never thrown through the air.

### Common pitfall

Reaching for `.unwrap()` to make the compiler stop complaining. It compiles, but you've traded a
handled error for a crash. Default to `?`; use `match` when you actually need to branch on the error.
