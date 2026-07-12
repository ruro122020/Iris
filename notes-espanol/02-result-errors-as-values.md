# `Result`: Los Errores Son Valores Ordinarios

🔑 Concepto Fundamental
Formato: el concepto, el modelo mental, las preguntas de comprobación y las respuestas que vale la pena recordar.

**Introducido mientras escribíamos:** la firma de `main` en `src/main.rs`, `Result<(), std::io::Error>`

### El código que lo introdujo

Iris arranca un servidor web, y arrancar un servidor puede fallar: puede que el puerto ya esté
ocupado. Por eso `main` se declara devolviendo un `Result`, y su última línea es `Ok(())`:

```rust
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // ... construir el router, enlazar un puerto, servir ...

    Ok(())
}
```

Dos cosas de esa firma merecen explicación: por qué una función anuncia sus fallos en su tipo de
retorno, y qué hace ese `Ok(())` en la última línea.

### El concepto

> **🔑 Concepto Fundamental: `Result`, los errores como valores ordinarios**
>
> Rust no tiene excepciones. Una función que puede fallar devuelve `Result<T, E>`: un **enum** (un tipo que es exactamente una de varias variantes enumeradas) con dos variantes, `Ok(T)` que lleva el valor de éxito, o `Err(E)` que lleva el error.
>
> El fallo forma parte de la *firma de tipos* de la función, y lo comprueba el compilador. No puedes olvidarte de gestionarlo como sí puedes olvidar un `try`/`catch`, porque el valor de éxito está encerrado dentro del envoltorio `Ok`, y la única manera de alcanzarlo es lidiar con ambas variantes.
>
> Nuestro `main` devuelve `Result<(), std::io::Error>`. El `()` es el **tipo unidad** (unit type), el valor de Rust que significa «aquí no hay nada relevante», parecido a `void`. Así que la firma se lee como «o bien tiene éxito sin nada que reportar, o bien falla con un error de E/S (entrada/salida)». Cuando `main` devuelve un `Result`, el código de salida del proceso pasa a ser 0 con `Ok` y distinto de cero con `Err` (imprimiendo el error), que es exactamente el contrato que esperan las herramientas de shell y systemd.
>
> Por eso la última línea es `Ok(())`: «llegué al final, tuve éxito, no hay nada que devolver». Sin punto y coma y sin `return`, porque en Rust la expresión final de un bloque *es* el valor del bloque.

El cambio de mentalidad respecto a Python/JS: en Python, `open("x.txt")` tiene una firma que no dice
nada sobre el fallo; descubres que puede lanzar un error solo por la documentación o por haberte
quemado en tiempo de ejecución. En Rust, `Result<File, io::Error>` dice «esto puede fallar» en
**tiempo de compilación, en la propia firma**. Los errores pasan de ser una sorpresa en tiempo de
ejecución a ser un hecho en tiempo de compilación.

### La pregunta obligada, y las cuatro herramientas que la responden

El valor que quieres (digamos un `File`) está **dentro** del `Ok(File)`, y el envoltorio podría estar
sosteniendo un `Err(e)` en su lugar. Antes de que el compilador te deje tocar el valor interior,
tienes que dar cuenta del `Err`. Intentar llamar a un método de `File` directamente sobre un
`Result<File, io::Error>` no compila. Cuatro maneras de responder a «¿qué quieres hacer primero con
el `Err`?»:

**1. `match`, la fundamental. Gestiona ambas variantes explícitamente.**

```rust
let file = match File::open("config.txt") {
    Ok(f) => f,
    Err(e) => {
        println!("could not open: {e}");
        return;
    }
};
```

`Ok(f)` y `Err(e)` son **patrones**: desestructuran el enum (extraen el valor interior) *y* a la vez
se ramifican según la variante. El compilador comprueba que el match sea **exhaustivo**; omite el
brazo `Err` y se negará a compilar. Esa exhaustividad es la garantía: un error no puede colarse.

**2. `?`, el modismo del día a día. Propaga el error hacia arriba.**

```rust
let file = File::open("config.txt")?;
```

Lee `?` como: «si es `Ok`, desenvuélvelo y continúa; si es `Err`, devuelve ese `Err` fuera de la
función actual ahora mismo». Solo compila dentro de una función cuyo tipo de retorno pueda contener
ese error (como `main` devolviendo `Result<(), io::Error>`). Esto es lo que escribirás casi siempre.

**3 y 4. `unwrap` y `expect`, las salidas de emergencia. Apuesta a que no fallará; entra en pánico si falla.**

```rust
let file = File::open("config.txt").unwrap();            // panic si es Err
let file = File::open("config.txt").expect("no config"); // panic con un mensaje
```

Un **panic** hace caer el hilo actual, desenrollando la pila (unwinding). Es contundente: convierte un
error recuperable en una caída. Aceptable solo en tests, código desechable, o un caso genuinamente
imposible (e incluso entonces, documenta por qué).

Modelo mental: `match` = gestionar explícitamente, `?` = propagarlo hacia arriba, `unwrap`/`expect` = apostar a que no ocurrirá, y entrar en pánico si ocurre.

### Preguntas de comprobación (y las respuestas que importan)

1. **Leyendo `Result<File, io::Error>` a secas, antes de ejecutar nada, ¿qué sabes que un
   programador de Python que llama a `open()` no puede saber por la firma?**
   Que la llamada puede fallar, y con qué tipo de error, en tiempo de compilación. Python oculta el
   fallo en la firma; te enteras por la documentación o en tiempo de ejecución.

2. **El `File` está dentro de `Ok(File)`. ¿Qué obliga el compilador en cada punto de llamada, y cuándo?**
   Debes dar cuenta del caso `Err` antes de alcanzar el valor interior, y se impone en tiempo de
   compilación. Llamar a un método de `File` directamente sobre un `Result` no compila. El único caso
   que nunca se te permite ignorar en silencio es el `Err`.

3. **`TcpListener::bind("127.0.0.1:3000").await?` y el puerto está ocupado. ¿Qué comportamiento
   ocurre, y dónde acaba el error?**
   El `?` ve el `Err` y lo devuelve fuera de `main`. La firma de `main` admite `Err(io::Error)`, así
   que el envoltorio bloqueante del runtime lo recibe, lo imprime, y el proceso sale con un código
   distinto de cero. El error es un valor devuelto hacia arriba por la cadena de llamadas, nunca algo
   lanzado por los aires.

### Error común

Recurrir a `.unwrap()` para que el compilador deje de quejarse. Compila, pero has cambiado un error
gestionado por una caída. Usa `?` por defecto; usa `match` cuando de verdad necesites ramificar según
el error.
