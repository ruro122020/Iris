---
title: Generics and Type Inference
date: 2026-07-17
description: Why "cannot infer type parameter" happens when a generic is built but never used, how to resolve it, and how to read what a generic function actually demands.
draft: false
---
# Generics and Type Inference

🔑 Core Concept
Format: the concept, the mental model, the check questions, and the answers worth remembering.

**Introduced while writing:** `src/main.rs`, the "cannot infer type parameter" error when building
`Router::new()` without yet serving it

### The code that triggered it

`main` built a router and returned. The router was never handed to a server, so `app` is created and
then abandoned:

```rust
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/on", post(turn_on))
        .route("/off", post(turn_off));

    Ok(())   // app is never used
}
```

This does not compile. Every reference below to "the error" means this one.

### The concept

> **🔑 Core Concept: generics and type inference**
>
> A generic type like `Router<S>` is a *template*, not a finished type. `S` stands in for "some type decided later." The compiler performs **monomorphization**: for whatever concrete `S` you end up using, it stamps out a specialized copy of `Router` with `S` filled in (for example `Router<()>`). This is a zero-cost abstraction, the generic disappears at compile time and you pay nothing at runtime for the flexibility.
>
> But to stamp out that copy, the compiler has to *know* what `S` is. It figures this out by **type inference**: looking at how you use the value and working backward. The problem in the code above is that nothing in it yet tells the compiler what `S` should be. You build the router and then just `Ok(())`, you never *use* `app` in a way that pins down the state type. So the compiler stops and says, in effect, "I can't infer `S`; tell me."

For `Router`, that `S` is the type of shared application state handlers can pull in (a database pool,
a config, and later for Iris the serial-port connection). The compiler cannot generate machine code
for a template; it must first fill `S` with a **concrete type**. The error is not a bug in the code,
it is the inference engine honestly reporting it lacks enough information *yet*.

### Two ways to resolve it

- **Use the value in a way that pins `S`.** Passing `app` to `axum::serve(...)` requires a router
  whose state type is `()` (the unit type, "no shared state"). That usage is the clue inference
  needs, so `S = ()` gets inferred and the error disappears on its own. This is the natural fix:
  serving the router is the real reason the state type resolves.
- **State the type yourself with an annotation.** `Router` written with no `<...>` uses its
  **default type parameter**, which the axum authors set to `()`:

  ```rust
  let app: Router = Router::new()   // Router means Router<()>
  ```

  Same answer inference would reach, just written explicitly.

Prefer letting real usage pin the type over adding an annotation only to delete it later.

### The four words, in plain language

The vocabulary is most of the difficulty here, so keep these definitions close:

- **Generic.** A type or function with a blank left in it. `Vec<T>` means "a growable list of
  *something*, and I am calling that something `T` until you tell me what it is." `Router<S>` means
  "a router carrying *some* shared state, called `S` until you tell me." The `<>` brackets hold the
  blanks.
- **Type inference.** The compiler filling in a blank for you by looking at how the value is *used*.
  You never had to say `Vec<i32>` if you push an `i32` into it one line later.
- **Monomorphization.** ("mono" = one, "morph" = shape.) Once the compiler knows what fills the
  blank, it writes out a private copy of the code with the blank permanently filled in. Fill it with
  `i32` and you get a list that only ever holds `i32`. Fill it with `String` and you get a second,
  separate copy. Two blanks, two copies. The generic version with the blank still in it never ships;
  it is a template, and templates do not ship.
- **Zero-cost abstraction.** A convenience that vanishes when compiled. You *wrote* one flexible
  `Router<S>`, but the machine code that runs is what you would have gotten by hand-writing each
  specialized router yourself. The flexibility cost nothing at runtime because it was all spent at
  compile time.

### How to find out what a function actually demands

Do not take anyone's word for it. Three ways to know, in increasing order of trust:

1. **Read the signature.** `axum::serve` never mentions `Router` by name. It asks for anything that
   behaves like a `Service` (axum's word for "a thing that takes a request and produces a response").
   The real rule is one layer down: axum implements `Service` only for `Router<()>`. A router still
   missing its state cannot answer a request, because a handler might ask for that state and there
   would be nothing to hand it. So `()` does not mean "no state." It means **"state, if any, is
   already supplied. Nothing is still owed."** `.with_state(port)` is what turns a
   `Router<SerialPort>` into a `Router<()>`.
2. **Read the docs for the exact version you have.** `cargo doc --open` builds HTML from the source
   of the exact `axum 0.8.9` pinned in `Cargo.lock`, so it can never be the wrong version the way a
   blog post can. Under `Router`, the **Trait Implementations** list is the ground truth for "what
   can I pass this to."
3. **Let the compiler tell you.** Guess, run `cargo check`, read the correction. Rust's trait errors
   say things like "the trait `Service` is not implemented for `Router<SerialPort>`" and add "note:
   required by a bound in `axum::serve`." This is the loop real Rust developers run, and it is the
   opposite of Python or JavaScript, where you learn you were wrong at runtime in production.
   **`cargo check` is not a grading step at the end. It is the conversation.**

### Check questions (and the answers that matter)

**1. Transfer it to a type you already know.** `Vec<T>` (Rust's growable array) is generic over its
element type `T`, exactly as `Router<S>` is generic over its state type. Predict each snippet:

```rust
// (a)
let v = Vec::new();

// (b)
let mut v = Vec::new();
v.push(5);
```

Does (a) compile on its own? Does (b)? If they differ, name *what changed* between them, using the
vocabulary of the concept rather than "the second one just works."

(a) does **not** compile. The reason is not `mut`, since `let v: Vec<i32> = Vec::new();` compiles
fine with no `mut` at all. The real error is `error[E0282]: type annotations needed for Vec<T>`. The
compiler is asking *a list of what?* Nothing in the program ever fills the blank, and it will not
guess.

(b) compiles because `v.push(5)` fills it: `5` is an `i32`, therefore `T = i32`. The blank got filled
**by how the value was used**, one line after it was created. (`mut` is also required in (b), but
that is a separate rule about mutation, not about inference.)

This is exactly the error in the `main.rs` at the top of this note: `Router::new()` has a blank, `app`
is never used, so nothing fills it.

**2. If Iris later had both a `Router<()>` and a `Router<SerialPort>`, how many versions of `Router`
exist in the binary, and does the program look up which kind it has at runtime?**

Two versions, one per concrete state type. But **no runtime lookup, and this is the part that is easy
to get wrong.** Monomorphization did not ship one flexible router that adapts. It shipped two
separate, rigid, hard-coded routers. The `Router<()>` code physically cannot handle a `SerialPort`;
that case was never compiled into it. So there is no decision left to make at runtime. The decision
was made at compile time and baked into the instructions.

That is precisely what **zero-cost abstraction** means: no check, no lookup, no branch. As fast as
two hand-written routers. The abstraction existed for the person typing, and it evaporated before the
program ever ran.

**3. Which direction does inference reason?**

Reading order and reasoning order are different. You *read* top to bottom:

```rust
let app = Router::new()... ;        // top: blank unfilled
axum::serve(listener, app).await?;  // bottom: serve demands Router<()>
```

The compiler does not start at the top and guess forward; it has no basis to. It collects a
constraint at the *bottom*: `serve` only accepts a router whose state type is `()`. `app` was passed
to `serve`. Therefore `app` is a `Router<()>`. Therefore the blank all the way back up at the top is
`()`.

Information flows **backward, from how a value is used to what that value must be.** The habit to
build: when wondering what type something is, do not look at where it was born. Look at where it is
consumed.

**4. Serving the router is what pins `S`. So what happens if you write everything *around* the serve
call but never make it?** Suppose `main` builds the router, opens a TCP listener, prints a startup
message, and stops there. The call that hands the router to the server, `axum::serve(listener, app)`,
is never written at all, so `app` is built and then abandoned:

```rust
let app = Router::new()
    .route("/health", get(health))
    .route("/on", post(turn_on))
    .route("/off", post(turn_off));

let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
println!("iris listening on http://127.0.0.1:3000");
// no axum::serve(...) call

Ok(())
```

Does `cargo check` pass? 

No, and it fails with the *same* "cannot infer type" error as before. Notice that every line actually
written is valid Rust: the listener is fine, the `println!` is fine. Validity is not the problem. The
listener and the `println!` never *touch* `app`, so nothing constrains `S`. **Type inference** has no
clue to work backward from, the blank stays empty, and the compiler refuses.

Which reframes the error entirely. It is not a bug that got introduced. It is an unfinished sentence,
and `axum::serve(listener, app)` is the word that finishes it. Nobody has to go back and *fix* the
router. Writing the serve call makes the error disappear on its own.

### Pitfall

Reaching for a type annotation the moment inference complains. Annotating `let app: Router` silences
the error, but the router still is not being served. When inference cannot resolve a type, the first question is not "what should I tell
the compiler?" but **"what am I not doing with this value yet?"**
