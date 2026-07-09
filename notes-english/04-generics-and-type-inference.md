# Generics and Type Inference

🔑 Core Concept
Format: the concept, the mental model, the check questions, and the answers worth remembering.

**Introduced while writing:** `src/main.rs`, the "cannot infer type parameter" error when building
`Router::new()` without yet serving it

### The concept

> **🔑 Core Concept: generics and type inference**
>
> A generic type like `Router<S>` is a *template*, not a finished type. `S` stands in for "some type decided later." The compiler performs **monomorphization**: for whatever concrete `S` you end up using, it stamps out a specialized copy of `Router` with `S` filled in (for example `Router<()>`). This is a zero-cost abstraction, the generic disappears at compile time and you pay nothing at runtime for the flexibility.
>
> But to stamp out that copy, the compiler has to *know* what `S` is. It figures this out by **type inference**: looking at how you use the value and working backward. The problem in 3a is that nothing in your code yet tells it what `S` should be. You build the router and then just `Ok(())`, you never *use* `app` in a way that pins down the state type. So the compiler stops and says, in effect, "I can't infer `S`; tell me."

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

### Check question (and the answer that matters)

_(comprehension check to be added and answered next)_
