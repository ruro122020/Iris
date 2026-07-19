---
title: Method Chaining by Ownership (the Builder Pattern)
date: 07-10-2026
description: Why `.route(...).route(...)` chains work: methods taking `self` by value consume the receiver and hand it back, plus the three receiver forms and what move semantics protect you from.
draft: false
---

# Method Chaining by Ownership (the Builder Pattern)

🔑 Core Concept
Format: the concept, the mental model, the check questions, and the answers worth remembering.

**Introduced while writing:** `src/main.rs` router construction in `main`

### The concept

> **🔑 Core Concept: the builder pattern and method chaining by ownership**
>
> Why can you write `.route(...).route(...).route(...)` in one chain? Because each `.route()` call **takes ownership of the router, adds the entry, and returns the router back**. Its signature is roughly `fn route(self, ...) -> Self`, note `self` by value, not `&self` by reference. So the call *consumes* the router it's called on and hands you back the (now bigger) router, which the next `.route()` in the chain consumes in turn.
>
> This is **move semantics** at work: the router isn't copied at each step, and it isn't shared; ownership flows down the chain from one call to the next. At runtime this compiles down to efficient in-place updates with no hidden allocations per link. The whole chain is one expression that starts with an empty `Router` and evaluates to the fully built one, which is why the final result lands in `app`.
>
> The alternative design, mutating in place, would look like `let mut app = Router::new(); app.route(...); app.route(...);` with a method taking `&mut self`. Rust libraries often prefer the consuming builder for construction because it reads as one expression and makes it impossible to accidentally use a half-built value. You'll see this `self`-consuming chain pattern all over the Rust ecosystem.

Applied to the Iris router:

```rust
let app = Router::new()          // an empty Router
    .route("/health", get(health))   // consumes it, returns Router
    .route("/on", post(turn_on))     // consumes that, returns Router
    .route("/off", post(turn_off));  // consumes that, returns the final Router
```

### `get(health)`: handlers passed as values

`health` with **no parentheses** is not a call; it is the function *itself*, passed as a value into
`get`. `get(health)` says "build a route rule that answers GET by running this handler." Compare:

- `get(health)` passes the function.
- `get(health())` would *call* `health` and pass its return value, which is not what a router wants.

Familiar from JavaScript callbacks like `arr.map(fn)`. Rust additionally checks at compile time that
`health`'s signature is actually usable as a handler.

### The three receiver forms (what `self` means)

When you call `router.route(...)`, the thing left of the dot is passed into the method as a special
first parameter named `self` (like `self` in Python or `this` in JavaScript). A method can ask to
receive it in three ways, and the choice decides what the caller may do afterward. Think of handing
someone your coffee cup:

| Form | Meaning | After the call |
|---|---|---|
| `&self` | "Look at my cup, give it back." Read-only borrow. | You still own it. Call as often as you like. |
| `&mut self` | "Take my cup, add sugar, give it back." Mutable borrow, changes in place. | You still own it. |
| `self` | "Here, keep my cup." Taken **by value**. | It is **gone from your hands**. |

`.route()` uses the third form: `fn route(self, ...) -> Self`. It takes the whole router by value and
returns a router back out (`Self` = "the type this method belongs to", here `Router`). You give up
the router, you receive a router back. That shape is what makes chaining work.

### Variables are name tags, not boxes

The trap is thinking `app` is a container that `.route()` reaches into and updates. It is not.
Separate two ideas English blurs:

- A **value**: the actual router data in memory.
- A **variable**: a *name tag* attached to a value.

```rust
let app = Router::new().route("/health", get(health));
let app2 = app.route("/on", post(turn_on));
println!("{:?}", app);
```

Following the code above in order: (1) the value is **moved out** from under the tag `app` and handed into the
method, so `app` now points at nothing and the compiler marks it "moved from"; (2) `.route()` runs
and **returns** a router; (3) `let app2 =` attaches the tag **`app2`** to that returned value.

**Nothing ever re-attached a value to `app`.** The return went to `app2` because that is where you
told it to go. Rust does not silently reassign `app`. To catch the result under the old name you must
say so, which is called **shadowing**:

```rust
let app = app.route("/on", post(turn_on));  // legal: rebind the name `app`
```

### Why the chain avoids this entirely

```rust
Router::new().route("/health", get(health)).route("/on", post(turn_on))
```

The intermediate routers have **no name tags at all**. They are anonymous values flowing straight
from one method's return into the next method's `self`. No variable is left behind pointing at
nothing, so there is nothing to accidentally reuse. That is exactly why the consuming builder is
comfortable to chain and awkward to split across statements.

### What the compiler is protecting you from

`.route()` owns the router and may restructure or free parts of it internally. If you could still
read the old `app`, you would be looking at memory the method may have moved out from under you. In
C++ that is **use-after-move**, a real bug class found at runtime if you are lucky. Rust makes it a
compile error.

### Check question (and the answer that matters)

1. **Given `.route()` takes `self` by value (it consumes the router), does this compile? Why?**

   ```rust
   let app = Router::new().route("/health", get(health));
   let app2 = app.route("/on", post(turn_on));
   println!("{:?}", app);   // using `app` after app.route(...) consumed it
   ```

   No. `app` no longer has a value: it was handed over to the `.route()` method, and the returned
   value was assigned to `app2`. Nothing re-attached a value to `app`. The compiler points at the
   `println!` line with `borrow of moved value: app`, and notes that the previous line moved it.
