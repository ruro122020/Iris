# Genéricos e Inferencia de Tipos

🔑 Concepto Fundamental
Formato: el concepto, el modelo mental, las preguntas de comprobación y las respuestas que vale la pena recordar.

**Introducido mientras escribíamos:** `src/main.rs`, el error "cannot infer type parameter" («no se
puede inferir el parámetro de tipo») al construir `Router::new()` sin llegar a servirlo todavía

### El código que lo provocó

`main` construía un router y retornaba. El router nunca se le entregaba a un servidor, así que `app`
se crea y luego se abandona:

```rust
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/on", post(turn_on))
        .route("/off", post(turn_off));

    Ok(())   // app nunca se usa
}
```

Esto no compila. Cada referencia a «el error» más abajo se refiere a este.

### El concepto

> **🔑 Concepto Fundamental: genéricos e inferencia de tipos**
>
> Un tipo genérico como `Router<S>` es una *plantilla*, no un tipo terminado. `S` hace de marcador de posición para «algún tipo que se decidirá más tarde». El compilador realiza la **monomorfización**: para el `S` concreto que acabes usando, estampa una copia especializada de `Router` con `S` ya rellenado (por ejemplo `Router<()>`). Esto es una abstracción de coste cero: el genérico desaparece en tiempo de compilación y no pagas nada en tiempo de ejecución por esa flexibilidad.
>
> Pero para estampar esa copia, el compilador tiene que *saber* qué es `S`. Lo averigua mediante la **inferencia de tipos**: mira cómo usas el valor y razona hacia atrás. El problema en el código de arriba es que todavía nada en él le dice al compilador cuál debería ser `S`. Construyes el router y luego simplemente haces `Ok(())`; nunca *usas* `app` de una forma que fije el tipo del estado. Así que el compilador se detiene y dice, en esencia, «no puedo inferir `S`; dímelo tú».

Para `Router`, ese `S` es el tipo del estado compartido de la aplicación que los handlers pueden
consultar (un pool de base de datos, una configuración, y más adelante, para Iris, la conexión al
puerto serie). El compilador no puede generar código máquina para una plantilla; primero debe rellenar
`S` con un **tipo concreto**. El error no es un bug en el código, es el motor de inferencia informando
honestamente de que le falta información suficiente *todavía*.

### Dos maneras de resolverlo

- **Usa el valor de una forma que fije `S`.** Pasar `app` a `axum::serve(...)` requiere un router
  cuyo tipo de estado sea `()` (el tipo unidad, «sin estado compartido»). Ese uso es la pista que la
  inferencia necesita, así que se infiere `S = ()` y el error desaparece por sí solo. Esta es la
  solución natural: servir el router es la razón real por la que se resuelve el tipo del estado.
- **Declara el tipo tú misma con una anotación.** `Router` escrito sin `<...>` usa su
  **parámetro de tipo por defecto**, que los autores de axum fijaron en `()`:

  ```rust
  let app: Router = Router::new()   // Router significa Router<()>
  ```

  La misma respuesta a la que llegaría la inferencia, solo que escrita explícitamente.

Es preferible dejar que el uso real fije el tipo antes que añadir una anotación solo para borrarla más tarde.

### Las cuatro palabras, en lenguaje llano

El vocabulario es la mayor parte de la dificultad aquí, así que ten estas definiciones a mano:

- **Genérico.** Un tipo o una función con un hueco en blanco. `Vec<T>` significa «una lista que puede
  crecer, de *algo*, y a ese algo lo llamo `T` hasta que me digas qué es». `Router<S>` significa «un
  router que lleva *algún* estado compartido, al que llamo `S` hasta que me digas cuál es». Los
  corchetes angulares `<>` contienen los huecos.
- **Inferencia de tipos.** El compilador rellenando un hueco por ti, mirando cómo se *usa* el valor.
  Nunca tuviste que escribir `Vec<i32>` si una línea después metes un `i32` dentro.
- **Monomorfización.** («mono» = uno, «morfo» = forma.) Una vez que el compilador sabe qué rellena el
  hueco, escribe una copia privada del código con el hueco rellenado de forma permanente. Rellénalo
  con `i32` y obtienes una lista que solo contiene `i32`. Rellénalo con `String` y obtienes una
  segunda copia, separada. Dos huecos, dos copias. La versión genérica, con el hueco todavía puesto,
  nunca llega al binario: es una plantilla, y las plantillas no se envían.
- **Abstracción de coste cero.** Una comodidad que se desvanece al compilar. Tú *escribiste* un único
  `Router<S>` flexible, pero el código máquina que se ejecuta es el mismo que habrías obtenido
  escribiendo a mano cada router especializado. La flexibilidad no costó nada en tiempo de ejecución
  porque se pagó entera en tiempo de compilación.

### Cómo averiguar qué exige realmente una función

No te fíes de la palabra de nadie. Tres maneras de saberlo, en orden creciente de confianza:

1. **Lee la firma.** `axum::serve` no menciona `Router` por su nombre en ninguna parte. Pide
   cualquier cosa que se comporte como un `Service` (la palabra de axum para «algo que toma una
   petición y produce una respuesta»). La regla real está una capa más abajo: axum implementa
   `Service` solo para `Router<()>`. Un router al que todavía le falta su estado no puede responder a
   una petición, porque un handler podría pedir ese estado y no habría nada que entregarle. Así que
   `()` no significa «sin estado». Significa **«el estado, si lo hay, ya está suministrado. No se
   debe nada»**. `.with_state(port)` es lo que convierte un `Router<SerialPort>` en un `Router<()>`.
2. **Lee la documentación de la versión exacta que tienes.** `cargo doc --open` genera el HTML a
   partir del código fuente del `axum 0.8.9` exacto que está fijado en `Cargo.lock`, así que nunca
   puede ser la versión equivocada, como sí puede serlo una entrada de blog. Dentro de `Router`, la
   lista de **Trait Implementations** es la fuente de la verdad sobre «a dónde puedo pasar esto».
3. **Deja que el compilador te lo diga.** Prueba, ejecuta `cargo check`, lee la corrección. Los
   errores de traits de Rust dicen cosas como "the trait `Service` is not implemented for
   `Router<SerialPort>`" y añaden "note: required by a bound in `axum::serve`". Este es el bucle que
   corren de verdad los desarrolladores de Rust, y es lo contrario de Python o JavaScript, donde te
   enteras de que estabas equivocada en tiempo de ejecución, en producción.
   **`cargo check` no es un paso de calificación al final. Es la conversación.**

### Preguntas de comprobación (y las respuestas que importan)

**1. Trasládalo a un tipo que ya conoces.** `Vec<T>` (el array que puede crecer de Rust) es genérico
sobre el tipo de sus elementos, `T`, exactamente igual que `Router<S>` es genérico sobre su tipo de
estado. Predice cada fragmento:

```rust
// (a)
let v = Vec::new();

// (b)
let mut v = Vec::new();
v.push(5);
```

¿Compila (a) por sí solo? ¿Y (b)? Si son distintos, di *qué cambió* entre ellos, usando el
vocabulario del concepto y no un «el segundo simplemente funciona».

(a) **no** compila. La razón no es `mut`, porque `let v: Vec<i32> = Vec::new();` compila
perfectamente sin ningún `mut`. El error real es `error[E0282]: type annotations needed for Vec<T>`.
El compilador está preguntando *¿una lista de qué?* Nada en el programa llega a rellenar el hueco, y
no va a adivinarlo.

(b) compila porque `v.push(5)` lo rellena: `5` es un `i32`, por lo tanto `T = i32`. El hueco se
rellenó **por la forma en que se usó el valor**, una línea después de crearlo. (`mut` también hace
falta en (b), pero esa es una regla aparte, sobre mutación, no sobre inferencia.)

Este es exactamente el error del `main.rs` del principio de esta nota: `Router::new()` tiene un
hueco, `app` nunca se usa, así que nada lo rellena.

**2. Si Iris tuviera más adelante tanto un `Router<()>` como un `Router<SerialPort>`, ¿cuántas
versiones de `Router` existen en el binario, y consulta el programa en tiempo de ejecución con cuál
de las dos está tratando?**

Dos versiones, una por cada tipo de estado concreto. Pero **ninguna consulta en tiempo de ejecución, y
esta es la parte que es fácil equivocar.** La monomorfización no envió un router flexible que se
adapta. Envió dos routers separados, rígidos, cableados. El código de `Router<()>` físicamente no
puede manejar un `SerialPort`; ese caso nunca se compiló dentro de él. Así que no queda ninguna
decisión que tomar en tiempo de ejecución. La decisión se tomó en tiempo de compilación y quedó
grabada en las instrucciones.

Eso es precisamente lo que significa **abstracción de coste cero**: ninguna comprobación, ninguna
consulta, ninguna bifurcación. Tan rápido como dos routers escritos a mano. La abstracción existió
para la persona que escribía, y se evaporó antes de que el programa llegara a ejecutarse.

**3. ¿En qué dirección razona la inferencia?**

El orden de lectura y el orden de razonamiento son cosas distintas. Tú *lees* de arriba abajo:

```rust
let app = Router::new()... ;        // arriba: el hueco sin rellenar
axum::serve(listener, app).await?;  // abajo: serve exige un Router<()>
```

El compilador no empieza arriba y adivina hacia adelante; no tiene base para hacerlo. Recoge una
restricción *abajo*: `serve` solo acepta un router cuyo tipo de estado sea `()`. A `serve` se le pasó
`app`. Por lo tanto `app` es un `Router<()>`. Por lo tanto el hueco, allá arriba del todo, es `()`.

La información fluye **hacia atrás, desde cómo se usa un valor hasta lo que ese valor debe ser.** El
hábito que hay que construir: cuando te preguntes de qué tipo es algo, no mires dónde nació. Mira
dónde se consume.

**4. Servir el router es lo que fija `S`. Entonces, ¿qué pasa si escribes todo lo que rodea a la
llamada a serve pero nunca la haces?** Supón que `main` construye el router, abre un listener TCP,
imprime un mensaje de arranque y se queda ahí. La llamada que le entrega el router al servidor,
`axum::serve(listener, app)`, no se escribe nunca, así que `app` se construye y luego se abandona:

```rust
let app = Router::new()
    .route("/health", get(health))
    .route("/on", post(turn_on))
    .route("/off", post(turn_off));

let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
println!("iris listening on http://127.0.0.1:3000");
// no hay ninguna llamada a axum::serve(...)

Ok(())
```

¿Pasa `cargo check`?

No, y falla con el *mismo* error de "cannot infer type" que antes. Fíjate en que cada línea que sí se
escribió es Rust válido: el listener está bien, el `println!` está bien. La validez no es el problema.
Ni el listener ni el `println!` llegan a *tocar* `app`, así que nada restringe `S`. La **inferencia de
tipos** no tiene ninguna pista desde la que razonar hacia atrás, el hueco se queda vacío, y el
compilador se niega.

Lo cual replantea el error por completo. No es un bug que se haya introducido. Es una frase sin
terminar, y `axum::serve(listener, app)` es la palabra que la termina. Nadie tiene que volver atrás a
*arreglar* el router. Escribir la llamada a serve hace que el error desaparezca solo.

### Trampa

Echar mano de una anotación de tipo en cuanto la inferencia se queja. Anotar `let app: Router` silencia
el error, pero el router sigue sin servirse. Cuando la inferencia no puede resolver un tipo, la primera
pregunta no es «¿qué debería decirle al compilador?», sino **«¿qué es lo que todavía no estoy haciendo
con este valor?»**
