---
title: Async Functions as State Machines
date: 07-05-2026
description: Why calling an `async fn` in Rust runs no code, and how the compiler turns the function body into a resumable state machine whose await points become enum states.
draft: false
---
# Study Log
## Rust Core Concept
Format: the concept, the mental model, the check questions, and the answers worth remembering.

## 1. `async fn`: a call builds a state machine, it doesn't run code

**Introduced while writing:** `src/main.rs` handlers (`health`, `turn_on`, `turn_off`)

### The code that introduced it

Iris is a web API with three route handlers. Every one of them is declared `async`, even though not
one of them awaits anything yet:

```rust
async fn health() -> &'static str {
    "OK\n"
}

async fn turn_on() -> &'static str {
    "led: on\n"
}

async fn turn_off() -> &'static str {
    "led: off\n"
}
```

The question that started this: if none of them wait for anything, why does axum insist they be
`async` at all? Answering it means knowing what `async` actually produces.

### The concept

> **🔑 Core Concept: `async fn` returns a state machine, not a value**
>
> Calling a normal `fn` runs its body immediately. Calling an `async fn` runs *nothing*: it instantly returns a **future**, a paused state-machine object describing work that *can* be done. The body only executes when something `.await`s it (or the runtime polls it). At compile time, Rust transforms the function body into an enum-like structure with one state per pause point. That's the whole trick, and it's why async needs no garbage collector or green-thread stacks: the "paused function" is just a plain struct sitting in memory. Our `health` has no pause points at all, so its state machine is trivial, but axum requires handlers to be async because *real* handlers will await things (like your serial port to the STM32 later).

```rust
// You write:
async fn greet() {
    let name = fetch_name().await;
    println!("Hello, {name}");
}

// Compiler roughly generates:
enum GreetStateMachine {
    Start,
    WaitingOnFetch { fut: FetchNameFuture },  // paused at .await
    Done,
}
```

### Pause points become enum states

Every `.await` inside an `async fn` is a **pause point**: a spot where the function may stop,
hand its thread back to the runtime, and need to be resumed later *with its local variables
intact*. Stack frames don't survive returning to the runtime, so the compiler rewrites the
function into an enum-like state machine at **compile time**, with one state per pause point:

```rust
async fn turn_on() -> &'static str {
    let port = open_serial().await;     // pause point 1
    port.send("LED ON").await;          // pause point 2
    "led: on\n"
}

// conceptually becomes:
enum TurnOnStateMachine {
    Start,                          // nothing run yet
    WaitingForSerial,               // parked at pause point 1
    WaitingForSend { port: Port },  // parked at pause point 2; port survives here
    Done,
}
```

Each state stores **only the locals that must survive across that pause**. Locals used and
finished before a pause never enter the enum: they live and die on the plain stack.

### Ownership: no special runtime-managed objects

`let f = turn_on();` makes `f` an **ordinary value** (an instance of the state-machine enum in
its `Start` state), obeying the exact same ownership rules as `let s = String::from("hi")`:

- `f` **owns** the future; when `f` goes out of scope it is dropped (freed) automatically, by scope rules.
- No garbage collector tracks it, and no runtime registry of "pending coroutines" exists
  (unlike JS promises, which are heap objects managed by the garbage collector).
- Drop an un-awaited future and the work it described simply never happens. It was only ever a value.
- Its size is known at compile time (the enum), so it needs no green-thread stack (Go's approach:
  kilobytes of growable stack per goroutine).

## Real World Example

**One thread executes one handler at a time**, so 8 worker threads means at most 8 handlers
*executing* at any instant, and that's true in both blocking and async worlds. In the
**blocking world**, a thread that hits a 5 ms wait stands inside the handler doing nothing for
5 ms; "in progress" equals "occupying a thread," and everyone else queues. In the **async
world**, at `.await` the task saves its state into the enum and is **parked**, the **thread is
released** and steps into another handler, and when the awaited thing completes the runtime
resumes the enum from its saved state on *any* free worker thread, not necessarily the original
one. Picture a restaurant: a blocking waiter stares at the kitchen until your food is up, while an
async waiter writes down the order (the enum state), serves six other tables, and returns when the
bell rings.

**The sentence to keep:** threads are only occupied by handlers that are **actively computing**,
never by handlers that are **waiting**. Tasks wait; threads don't.

### Check questions (and the answers that matter)

1. **Call `turn_on()` but never `.await` it. Does "led: on" ever get produced?**
   No. The call only constructs the state machine in `Start`. No `.await`, no polling, no execution.

2. **`let f = turn_on();` What is `f`, and who is responsible for its memory?**
   An instance of the state-machine enum, sitting in its `Start` state. The parentheses in
   `turn_on()` mean the function really *was called*, so `f` is a value the call returned, not a
   name for the function itself. It is just that the returned value is a paused state machine rather
   than `"led: on\n"`. `f` owns it, and scope rules free it. Same rules as a `String`.

3. **Which locals get stored in the enum here, and which do not?**

   ```rust
   async fn example() {
       let a = 1;
       let b = a + 1;
       println!("{b}");          // a and b used, then never again
       let msg = "LED ON";
       send(msg).await;          // pause point
       println!("sent {msg}");   // msg used after the pause
   }
   ```

   Only `msg`, because it is used *across* the pause and so has to survive it. `a` and `b` are
   finished with before the pause is ever reached, so they live and die on the plain stack and never
   enter the enum.

4. **1000 simultaneous requests, each awaiting a 5 ms serial reply, 8 worker threads. How many
   tasks can be "waiting on serial" at once, and what does each consume?**
   All ~992 non-executing ones can wait *simultaneously*; there is no cap on waiting because
   waiting costs no thread. Each parked task consumes only the bytes of its enum (tens to hundreds
   of bytes). A thousand parked threads would cost megabytes of stack each; a thousand parked
   tasks cost roughly a few Strings' worth of memory. That asymmetry is why async exists for servers.

### Common pitfall 

Forgetting `.await` is a classic: the code compiles, nothing happens. The compiler emits a
warning, `unused implementer of Future that must be used: futures do nothing unless you
.await or poll them`. Read your warnings.
