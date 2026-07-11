# Registro de Estudio
## Concepto Fundamental de Rust
Formato: el concepto, el modelo mental, las preguntas de comprobación y las respuestas que vale la pena recordar.

## 1. `async fn`: una llamada construye una máquina de estados, no ejecuta código

**Introducido mientras escribíamos:** los handlers de `src/main.rs` (`health`, `turn_on`, `turn_off`)

### El concepto

> **🔑 Concepto Fundamental: `async fn` devuelve una máquina de estados, no un valor**
>
> Llamar a una `fn` normal ejecuta su cuerpo inmediatamente. Llamar a una `async fn` no ejecuta *nada*: devuelve al instante un **future** (futuro), un objeto de máquina de estados en pausa que describe un trabajo que *se puede* hacer. El cuerpo solo se ejecuta cuando algo le hace `.await` (o cuando el runtime lo sondea, «poll»). En tiempo de compilación, Rust transforma el cuerpo de la función en una estructura tipo enum con un estado por cada punto de pausa. Ese es todo el truco, y es la razón por la que async no necesita recolector de basura ni pilas de hilos verdes: la «función pausada» no es más que un struct normal y corriente en memoria. Nuestro `health` no tiene ningún punto de pausa, así que su máquina de estados es trivial, pero axum exige que los handlers sean async porque los handlers *de verdad* van a esperar cosas (como tu puerto serie hacia el STM32 más adelante).

```rust
// Tú escribes:
async fn greet() {
    let name = fetch_name().await;
    println!("Hello, {name}");
}

// El compilador genera, a grandes rasgos:
enum GreetStateMachine {
    Start,
    WaitingOnFetch { fut: FetchNameFuture },  // pausado en el .await
    Done,
}
```

### Los puntos de pausa se convierten en estados del enum

Cada `.await` dentro de una `async fn` es un **punto de pausa**: un lugar donde la función puede
detenerse, devolver su hilo al runtime y necesitar que la reanuden más tarde *con sus variables
locales intactas*. Los marcos de pila (stack frames) no sobreviven al retorno hacia el runtime, así
que el compilador reescribe la función como una máquina de estados tipo enum en **tiempo de
compilación**, con un estado por cada punto de pausa:

```rust
async fn turn_on() -> &'static str {
    let port = open_serial().await;     // punto de pausa 1
    port.send("LED ON").await;          // punto de pausa 2
    "led: on\n"
}

// conceptualmente se convierte en:
enum TurnOnStateMachine {
    Start,                          // todavía no se ha ejecutado nada
    WaitingForSerial,               // aparcado en el punto de pausa 1
    WaitingForSend { port: Port },  // aparcado en el punto de pausa 2; aquí sobrevive `port`
    Done,
}
```

Cada estado guarda **únicamente las variables locales que deben sobrevivir a esa pausa**. Las
locales que se usan y se terminan antes de una pausa nunca entran en el enum: viven y mueren en la
pila normal.

### Ownership (propiedad): no hay objetos especiales gestionados por el runtime

`let f = turn_on();` convierte a `f` en un **valor ordinario** (una instancia del enum de la máquina
de estados en su estado `Start`), que obedece exactamente las mismas reglas de ownership que
`let s = String::from("hi")`:

- `f` **posee** el future; cuando `f` sale de ámbito (scope) se descarta (se libera) automáticamente, por las reglas de ámbito.
- Ningún recolector de basura lo rastrea, y no existe ningún registro en el runtime de «corrutinas pendientes»
  (a diferencia de las promesas de JS, que son objetos en el heap gestionados por el recolector de basura).
- Si descartas un future al que nunca le hiciste `.await`, el trabajo que describía sencillamente nunca ocurre. Nunca fue más que un valor.
- Su tamaño se conoce en tiempo de compilación (el enum), así que no necesita una pila de hilo verde (el enfoque de Go:
  kilobytes de pila expandible por cada goroutine).

## Ejemplo del Mundo Real

**Un hilo ejecuta un handler a la vez**, así que 8 hilos de trabajo (worker threads) significan como
máximo 8 handlers *ejecutándose* en cualquier instante, y eso es cierto tanto en el mundo bloqueante
como en el async. En el **mundo bloqueante**, un hilo que se topa con una espera de 5 ms se queda
parado dentro del handler sin hacer nada durante 5 ms; «en progreso» equivale a «ocupando un hilo»,
y todos los demás hacen cola. En el **mundo async**, en el `.await` la tarea guarda su estado en el
enum y queda **aparcada**, el **hilo queda liberado** y pasa a otro handler, y cuando lo que se
esperaba se completa, el runtime reanuda el enum desde su estado guardado en *cualquier* hilo de
trabajo libre, no necesariamente el original. Piensa en la analogía del restaurante: un camarero
bloqueante se queda mirando la cocina hasta que tu comida está lista, mientras que un camarero async
apunta el pedido (el estado del enum), atiende otras seis mesas y vuelve cuando suena la campana.

**La frase que hay que retener:** los hilos solo están ocupados por handlers que están **computando
activamente**, nunca por handlers que están **esperando**. Las tareas esperan; los hilos no.

### Preguntas de comprobación (y las respuestas que importan)

1. **Llamas a `turn_on()` pero nunca le haces `.await`. ¿Se llega a producir alguna vez "led: on"?**
   No. La llamada solo construye la máquina de estados en `Start`. Sin `.await`, no hay polling ni ejecución.

2. **`let f = turn_on();` ¿Qué es `f`, y quién es responsable de su memoria?**
   Una instancia del enum de la máquina de estados (no «una función declarada»: los paréntesis
   significan que la función *fue llamada* y devolvió este valor). `f` lo posee; las reglas de ámbito
   lo liberan. Las mismas reglas que para un `String`.

3. **En la fn `example()` con `a` y `b` antes del `.await` y `msg` usado después, ¿qué variables
   locales se guardan en el enum?**
   Solo `msg` (usada a través de la pausa). `a` y `b` terminan antes de la pausa: pila normal, nunca se guardan.

   ```rust
    async fn example() {
      let a = 1;
      let b = a + 1;
      println!("{b}");          // se usan `a` y `b`, y ya nunca más
      let msg = "LED ON";
      send(msg).await;          // punto de pausa
      println!("sent {msg}");   // `msg` se usa después de la pausa
    }
   ```

4. **1000 peticiones simultáneas, cada una esperando una respuesta serie de 5 ms, 8 hilos de trabajo.
   ¿Cuántas tareas pueden estar «esperando el puerto serie» a la vez, y qué consume cada una?**
   Las ~992 que no se están ejecutando pueden esperar *simultáneamente*; no hay límite a la espera
   porque esperar no cuesta ningún hilo. Cada tarea aparcada consume solo los bytes de su enum
   (decenas o centenares de bytes). Mil hilos aparcados costarían megabytes de pila cada uno; mil
   tareas aparcadas cuestan aproximadamente lo que unos pocos Strings de memoria. Esa asimetría es la
   razón por la que async existe para los servidores.

### Error común

Olvidar el `.await` es un clásico: el código compila y no pasa nada. El compilador emite un aviso,
`unused implementer of Future that must be used: futures do nothing unless you .await or poll them`
(«implementador de Future sin usar que debe usarse: los futures no hacen nada a menos que les hagas
`.await` o los sondees»). Lee tus avisos.
