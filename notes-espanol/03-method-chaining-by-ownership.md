# Encadenamiento de Métodos por Ownership (el Patrón Builder)

🔑 Concepto Fundamental
Formato: el concepto, el modelo mental, las preguntas de comprobación y las respuestas que vale la pena recordar.

**Introducido mientras escribíamos:** la construcción del router en `main`, en `src/main.rs`

### El concepto

> **🔑 Concepto Fundamental: el patrón builder y el encadenamiento de métodos por ownership**
>
> ¿Por qué puedes escribir `.route(...).route(...).route(...)` en una sola cadena? Porque cada llamada a `.route()` **toma posesión (ownership) del router, añade la entrada, y devuelve el router de vuelta**. Su firma es aproximadamente `fn route(self, ...) -> Self`; fíjate en que es `self` por valor, no `&self` por referencia. Así que la llamada *consume* el router sobre el que se la invoca y te devuelve el router (ahora más grande), que la siguiente `.route()` de la cadena consume a su vez.
>
> Esto son las **move semantics** (semánticas de movimiento) en acción: el router no se copia en cada paso, ni se comparte; la propiedad fluye por la cadena de una llamada a la siguiente. En tiempo de ejecución esto se compila a actualizaciones eficientes en el sitio (in-place), sin asignaciones de memoria ocultas por cada eslabón. La cadena entera es una única expresión que empieza con un `Router` vacío y evalúa al router completamente construido, y por eso el resultado final aterriza en `app`.
>
> El diseño alternativo, mutar en el sitio, se vería así: `let mut app = Router::new(); app.route(...); app.route(...);` con un método que toma `&mut self`. Las librerías de Rust suelen preferir el builder consumidor para la construcción porque se lee como una sola expresión y hace imposible usar accidentalmente un valor a medio construir. Verás este patrón de cadena que consume `self` por todo el ecosistema de Rust.

Aplicado al router de Iris:

```rust
let app = Router::new()          // un Router vacío
    .route("/health", get(health))   // lo consume, devuelve un Router
    .route("/on", post(turn_on))     // consume ese, devuelve un Router
    .route("/off", post(turn_off));  // consume ese, devuelve el Router final
```

### `get(health)`: handlers pasados como valores

`health` **sin paréntesis** no es una llamada; es la función *en sí misma*, pasada como un valor
dentro de `get`. `get(health)` dice «construye una regla de ruta que responda a GET ejecutando este
handler». Compara:

- `get(health)` pasa la función.
- `get(health())` *llamaría* a `health` y pasaría su valor de retorno, que no es lo que un router quiere.

Te resultará familiar de los callbacks de JavaScript como `arr.map(fn)`. Rust además comprueba en
tiempo de compilación que la firma de `health` sea realmente utilizable como handler.

### Las tres formas del receptor (qué significa `self`)

Cuando llamas a `router.route(...)`, la cosa que está a la izquierda del punto se pasa al método como
un primer parámetro especial llamado `self` (como `self` en Python o `this` en JavaScript). Un método
puede pedir recibirlo de tres maneras, y la elección decide qué puede hacer el llamador después.
Piensa en darle a alguien tu taza de café:

| Forma | Significado | Después de la llamada |
|---|---|---|
| `&self` | «Mira mi taza y devuélvemela.» Préstamo de solo lectura. | Sigues siendo el dueño. Llama tantas veces como quieras. |
| `&mut self` | «Coge mi taza, échale azúcar y devuélvemela.» Préstamo mutable, cambia en el sitio. | Sigues siendo el dueño. |
| `self` | «Toma, quédate mi taza.» Tomada **por valor**. | **Ha desaparecido de tus manos.** |

`.route()` usa la tercera forma: `fn route(self, ...) -> Self`. Toma el router entero por valor y
devuelve un router de vuelta (`Self` = «el tipo al que pertenece este método», aquí `Router`).
Entregas el router y recibes un router de vuelta. Esa forma es lo que hace posible el encadenamiento.

### Las variables son etiquetas con nombre, no cajas

La trampa es pensar que `app` es un contenedor dentro del cual `.route()` mete la mano para
actualizarlo. No lo es. Separa dos ideas que el lenguaje corriente confunde:

- Un **valor**: los datos reales del router en memoria.
- Una **variable**: una *etiqueta con nombre* adherida a un valor.

```rust
let app = Router::new().route("/health", get(health));
let app2 = app.route("/on", post(turn_on));
println!("{:?}", app);
```

Siguiendo el código anterior en orden: (1) el valor es **movido fuera** de debajo de la etiqueta `app`
y entregado al método, así que `app` ahora no apunta a nada y el compilador la marca como «movida»
(moved from); (2) `.route()` se ejecuta y **devuelve** un router; (3) `let app2 =` adhiere la etiqueta
**`app2`** a ese valor devuelto.

**Nada volvió a adherir jamás un valor a `app`.** El retorno fue a `app2` porque ahí es donde le
dijiste que fuera. Rust no reasigna `app` en silencio. Para recoger el resultado bajo el nombre
antiguo tienes que decirlo, y a eso se le llama **shadowing** (ensombrecimiento):

```rust
let app = app.route("/on", post(turn_on));  // legal: reasigna el nombre `app`
```

### Por qué la cadena evita todo esto

```rust
Router::new().route("/health", get(health)).route("/on", post(turn_on))
```

Los routers intermedios **no tienen ninguna etiqueta con nombre**. Son valores anónimos que fluyen
directamente desde el retorno de un método hacia el `self` del siguiente. No queda ninguna variable
atrás apuntando a nada, así que no hay nada que se pueda reutilizar por accidente. Eso es exactamente
por qué el builder consumidor resulta cómodo de encadenar e incómodo de partir en varias sentencias.

### De qué te está protegiendo el compilador

`.route()` posee el router y puede reestructurarlo o liberar partes de él internamente. Si pudieras
seguir leyendo el viejo `app`, estarías mirando memoria que el método puede haber movido por debajo
de tus pies. En C++ eso es **use-after-move** (uso después de mover), una clase de bug real que
encuentras en tiempo de ejecución si tienes suerte. Rust lo convierte en un error de compilación.

### Pregunta de comprobación (y la respuesta que importa)

1. **Dado que `.route()` toma `self` por valor (consume el router), ¿compila esto? ¿Por qué?**

   ```rust
   let app = Router::new().route("/health", get(health));
   let app2 = app.route("/on", post(turn_on));
   println!("{:?}", app);   // usando `app` después de que app.route(...) la consumiera
   ```

   No. `app` ya no tiene un valor: fue entregada al método `.route()`, y el valor devuelto se asignó a
   `app2`. Nada volvió a adherir un valor a `app`. El compilador señala la línea del `println!` con
   `borrow of moved value: app` («préstamo de un valor movido: app»), y apunta que la línea anterior lo movió.
