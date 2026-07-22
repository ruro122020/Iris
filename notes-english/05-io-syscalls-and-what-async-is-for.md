---
title: I/O, Syscalls, and What Async Is Actually For
date: 2026-07-19
description: What a syscall is and why the kernel sits between your program and the hardware. Also why I/O does not mean slow, and why the real waiting happens inside the accept loop rather than in code you write.
draft: false
---

# I/O, Syscalls, and What `async` Is Actually For

🔑 Core Concept
Format: the concept, the mental model, the check questions, and the answers worth remembering.

**Introduced while writing:** the listener and serve loop in `src/main.rs`

### The code that introduced it

Iris is a small Rust web API built on axum. Its `main` builds a router, opens a network port, and
then serves requests on that port forever. The three lines that open the port and serve look like
this:

```rust
let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
println!("iris listening on http://127.0.0.1:3000");
axum::serve(listener, app).await?;
```

The question that started this: `bind` is an `async fn`, so you have to `.await` it. Why? What is it
waiting *for*? Answering that means being precise about what I/O is, and about which part of a
server is actually slow.

### The concept

> **🔑 Core Concept: I/O, syscalls, and what `async` is actually for**
>
> **I/O (Input/Output)** is any operation where your program exchanges data with something outside its own memory: disk, network, keyboard, another process, a serial port. The contrast is **computation**, which happens entirely inside the CPU and RAM. The distinction matters because of **who controls the clock**. Computation is bounded, `a + b` takes nanoseconds and nothing can slow it down. I/O is unbounded, a network packet arrives when it arrives and your program has no say. `async` exists specifically for the second category.
>
> Your program is not allowed to touch the network card. The operating system owns the hardware, and your process runs in **user mode**, walled off from it. The only way through the wall is a **syscall (system call)**: a controlled doorway where you ask the kernel to do something on your behalf. "Talking to the OS" means crossing that doorway.
>
> But being I/O does not automatically mean being slow, and conflating the two is the mistake to avoid. `TcpListener::bind` is I/O by category, which is why it returns `io::Error`, but it is not a long wait: it asks the kernel to reserve the port and returns in microseconds, success or failure. The genuinely unbounded wait is `accept`, "sleep until some client decides to connect to me," which could be a millisecond or next Tuesday. **`bind` is `async` for API uniformity and to register the socket with the runtime's event loop, not because it is slow.** The slowness that justifies the entire async machine lives in `accept`, inside the serve loop, not in `bind`.

### The wall between your program and the hardware

Two terms worth spelling out, because the rest only makes sense once they are concrete.

**User mode** is the restricted privilege level your program runs in. It can read and write its own
memory and do arithmetic, and that is roughly it. It cannot address the network card, the disk
controller, or another process's memory. The CPU itself enforces this in hardware.

**Kernel mode** is the privileged level the operating system runs in, where touching hardware is
allowed. The **kernel** is the core of the OS: the part that owns the hardware and hands out access
to it.

A **syscall (system call)** is the doorway between them. Your program loads up a request ("open a
socket on port 3000"), triggers a special CPU instruction, and control transfers to the kernel. The
kernel does the privileged work, then hands control back. It is not a function call to a library in
your own process; it is a controlled crossing into code that has powers you do not.

So `TcpListener::bind` is not really "Rust code that opens a port." It is Rust code that asks the
kernel to open a port. Three syscalls, roughly: `socket` (make me an endpoint), `bind` (reserve this
address and port for it), `listen` (start queueing incoming connections).

### The mistake to avoid: I/O does not mean slow

It is tempting to build the rule "I/O, therefore `async`, therefore it waits a long time." The first
two links hold. The third does not.

| Operation | I/O? | Actually a long wait? | Why |
|---|---|---|---|
| `bind` | Yes | **No** | Reserves a port. The kernel answers immediately: yes or the port is taken. |
| `accept` | Yes | **Yes, unbounded** | Waits for a *client on the network* to connect. Nothing local controls when that happens. |
| `read` from a socket | Yes | Yes, unbounded | Waits for bytes that another machine has to send you. |
| `a + b` | No | No | Pure computation, entirely inside the CPU. |

`bind` fails fast and succeeds fast. If the port is free you have it in microseconds. If another
process already holds port 3000, the kernel tells you so immediately, and that is what comes back as
`Err(io::Error)`. There is no waiting involved in either outcome. The only thing you are ever
"waiting" on is a syscall that returns right away.

So why is `bind` an `async fn` at all? Two honest reasons, neither of which is speed:

1. **API uniformity.** Tokio's `TcpListener` is an async type. Every method on it is async, so you
   learn one calling convention instead of remembering which handful of methods secretly are not.
2. **Registration with the event loop.** The runtime keeps a **reactor**, an event loop built on the
   OS's readiness-notification facility (`epoll` on Linux, `kqueue` on macOS, IOCP on Windows). It
   is the thing that gets told "this socket now has a connection waiting." Creating the listener is
   the moment the socket gets registered there, so that later, when you *do* wait on `accept`, the
   runtime already knows how to be woken up about it.

### Where the real wait lives

Notice what you do **not** write anywhere in Iris: a call to `accept`. That is not an oversight. The
accept loop lives inside `axum::serve`:

```rust
axum::serve(listener, app).await?;   // this .await never finishes under normal operation
```

Conceptually `serve` is doing this forever:

```
loop {
    let connection = listener.accept().await;   // <-- the unbounded wait, right here
    spawn a task that hands `connection` to the router
}
```

That `.await` on `accept` is the one that parks. That is where the whole async design pays for
itself: a thousand idle connections cost you a thousand parked state machines (tens of bytes each),
not a thousand parked OS threads (megabytes of stack each).

And that is why `main` appears to hang on the `axum::serve` line. It is not stuck. It is doing its
job. It will sit there until you press Ctrl+C.

### Why the `println!` is not decoration

```rust
println!("iris listening on http://127.0.0.1:3000");
```

A server that is working correctly and a server that is frozen look **identical from the outside**:
both just sit there producing nothing. The print is the only way to tell the two apart. You emit it
the instant you know the port is genuinely open, so that silence afterward means "waiting for you,"
not "broken."

`127.0.0.1` is **localhost**, meaning your own machine and nothing else. Nothing outside your
computer can reach this server. That is deliberate.

### Check questions (and the answers that matter)

1. **`bind` returns a `Result` and has to be `.await`ed. Which of those two facts is about *failure*,
   and which is about *waiting*, and is either one telling you it is slow?**
   The `Result` is about failure: the port may already be taken, so the call can come back
   `Err(io::Error)`. The `.await` is about it being an async API, not about it being slow. Neither
   fact says `bind` is slow, because it is not. It answers in microseconds either way.

2. **Your program never calls `accept` anywhere in its source. So where does the server actually wait
   for a client to connect, and what is parked while it waits?**
   Inside `axum::serve`, which runs an accept loop for you. The `.await` on `accept` parks a state
   machine (a few dozen bytes), not an OS thread (megabytes of stack). That asymmetry is the entire
   reason async exists for servers.

3. **A program computes a 10-second hash in a handler, with no `.await` anywhere in it. Is that I/O?
   Will `async` help?**
   No and no. It is pure computation, entirely inside the CPU and RAM, so it is not I/O at all.
   `async` only helps you stop *occupying a thread while waiting on something external*. There is no
   waiting here, only working, and the work has to happen on some thread regardless. Async is the
   wrong tool; a dedicated thread pool for blocking or CPU-heavy work is the right one.

4. **Why can your Rust program not simply write to the network card directly and skip the syscall?**
   Because the CPU enforces a privilege split. Your process runs in user mode and is not permitted to
   address hardware; the kernel runs in kernel mode and is. The syscall is the only controlled
   doorway between them. This is what keeps one buggy program from scribbling over another program's
   network traffic or the disk.

### Common pitfall

Reasoning "it is `async`, so it must be slow, so I should worry about it." Asynchrony marks
*potential* waiting, not actual waiting. Plenty of async functions return immediately. Conversely,
plenty of genuinely slow work (a long computation) is not async at all and gets no help from async.
Ask what the operation is actually waiting on, and whether anything outside your process controls
when it finishes.
