<p align="center">
  <picture>
    <source media="(max-width: 600px) and (prefers-color-scheme: light)" srcset=".github/assets/banner-phone-light.svg">
    <source media="(max-width: 600px)" srcset=".github/assets/banner-phone.svg">
    <source media="(prefers-color-scheme: light)" srcset=".github/assets/banner-light.svg">
    <img src=".github/assets/banner.svg" width="100%" alt="Laconic. Primero la respuesta. Hecho para conservar los hechos. Inglés australiano. Pixel art de un casco espartano con cresta junto al nombre Laconic en letras de bloque con remates.">
  </picture>
</p>

<div align="center">

[![tests](https://github.com/yungSNAT/Laconic/actions/workflows/tests.yml/badge.svg?branch=main)](https://github.com/yungSNAT/Laconic/actions/workflows/tests.yml) [![Codecov](https://codecov.io/gh/yungSNAT/Laconic/graph/badge.svg)](https://codecov.io/gh/yungSNAT/Laconic) [![CodeQL](https://github.com/yungSNAT/Laconic/actions/workflows/codeql.yml/badge.svg?branch=main)](https://github.com/yungSNAT/Laconic/actions/workflows/codeql.yml) [![OSV-Scanner](https://github.com/yungSNAT/Laconic/actions/workflows/osv-scanner.yml/badge.svg?branch=main)](https://github.com/yungSNAT/Laconic/actions/workflows/osv-scanner.yml) [![SonarCloud](https://sonarcloud.io/api/project_badges/measure?project=EquityBot_Laconic&metric=alert_status)](https://sonarcloud.io/summary/new_code?id=EquityBot_Laconic) [![Codacy code quality](https://app.codacy.com/project/badge/Grade/623f5ab312d4497c9e8161e547c95f44)](https://app.codacy.com/gh/yungSNAT/Laconic/dashboard)<br>
[![OpenSSF Scorecard](https://raw.githubusercontent.com/ryanduguid/.github/badges/yungSNAT/Laconic.svg)](https://scorecard.dev/viewer/?uri=github.com/yungSNAT/Laconic) [![Procedencia de compilación SLSA](.github/assets/badges/slsa.svg)](https://github.com/yungSNAT/Laconic/attestations) [![VirusTotal](.github/assets/badges/virustotal.svg)](https://github.com/yungSNAT/Laconic/releases/latest)<br>
[![release](https://img.shields.io/github/v/release/yungSNAT/Laconic?filter=laconic-check-v*&color=66023C&labelColor=000000&style=flat)](https://github.com/yungSNAT/Laconic/releases?q=laconic-check-v&expanded=true) [![licence: Apache-2.0](.github/assets/badges/licence.svg)](LICENSE) [![fork of: i-have-adhd](.github/assets/badges/fork-of.svg)](https://github.com/ayghri/i-have-adhd)

[Instalación](#install) · [Uso](#use) · [Qué cambia](#what-changes) · [Evaluación](#evaluation) · [Comprobador](#checker) · [Siempre activo](#always-on) · [Desarrollo](#development) · [Créditos](#credits) · [Licencia](#licence)

[English](README.md) · Español · [한국어](README.ko.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md)

</div>

Laconic es un 'humanizador' de IA y una skill 'sin relleno de IA' (no AI slop) para agentes de programación y asistentes de chat. Hace que el modelo dé primero la respuesta, marque lo que no ha verificado y se detenga cuando la respuesta está completa, y le indica que conserve cada hecho, paso y salvedad que el lector necesita. Le indica que escriba cada frase original según el Australian Government Style Manual, con las excepciones de la casa que enumera la skill.

Para humanizar un texto escrito por IA, pide al agente que lo humanice. La skill de prosa elimina los patrones de escritura de IA, está escrita para conservar cada hecho, número y salvedad, y deja el texto más corto, sin rellenarlo con una personalidad impostada. El [benchmark de supervivencia de hechos](evals/README.md#fact-survival-benchmark) que pone a prueba esta promesa aún no tiene resultados.

<table>
<tr>
<td align="center" width="33%"><h3>4.63 frente a 4.07</h3>frente a la simple 'Answer concisely.'<br><sub><a href="#evaluation">rúbrica de la casa, 45 casos; 6 hallazgos bloqueantes frente a 29</a></sub></td>
</tr>
</table>

La cifra viene del propio arnés de Laconic el 20 de octubre de 2026, con un único modelo generador. [Evaluación](#evaluation) da los límites y los comandos para repetirla.

<a id="install"></a>
## Instalación

Pega esto en tu agente:

```text
Install the laconic skill/plugin from https://github.com/yungSNAT/laconic, following the repository's AGENTS.md.
```

O sigue las [instrucciones de instalación](INSTALL.md) para tu agente. Para Claude Code:

```bash
claude plugin marketplace add yungSNAT/laconic
claude plugin install laconic@laconic
```

Para cualquier agente compatible con la Skills CLI:

```bash
npx skills add yungSNAT/laconic --global --agent "*" --yes
```

<a id="use"></a>
## Uso

Reinicia el agente y escribe `/laconic`. Sigue activo durante la sesión hasta que digas 'stop laconic'.

<a id="what-changes"></a>
## Qué cambia

<table>
<tr>
<td width="50%">

### Antes

> Great question! Let me take a look. Your auth flow has a few moving pieces: the middleware, the token verification and the cookie handling. It's likely a caching issue, which typically resolves itself within an hour or so. Hope this helps! Let me know if you want to dig deeper.

</td>
<td width="50%">

### Después

> The 401 comes from the missing `Authorization` header: the endpoint requires a bearer token and the request sends none.
>
> 1. Add `Authorization: Bearer <token>` at `src/api.ts:42`
> 2. Run `npm test -- api.spec.ts`
>
> Not verified: whether the token expires mid-session. The logs do not show it.

</td>
</tr>
</table>

Los ejemplos se dejan en inglés, la lengua en que escribe Laconic.

<a id="what-it-includes"></a>
## Qué incluye

| Pieza | Ruta | Cuándo se aplica |
|---|---|---|
| Modo sesión | `skills/laconic/SKILL.md` | Tras escribir `/laconic`, durante el resto de la sesión, hasta 'stop laconic' |
| Skill de prosa | `skills/laconic-prose/SKILL.md` | La carga el modelo para redactar, editar y auditar prosa; afecta solo a esa tarea |
| Reglas compactas | `rules/laconic.md` | Siempre cargadas, si las añades a las reglas de tu agente (unas 240 palabras) |

Las reglas de sesión, completas en [SKILL.md](skills/laconic/SKILL.md):

1. Primero la respuesta, luego la justificación.
2. Di qué sabes y cómo. Marca lo no verificado; 'no puedo juzgarlo con la evidencia que tengo' es una respuesta aceptable.
3. La longitud depende de lo que el lector debe decidir, hacer o entender. Recorta palabras, nunca contenido.
4. Numera los procedimientos.
5. Céntrate en la tarea actual.
6. Mantén visible el estado.
7. Expón los errores con claridad: el error, la causa conocida y la solución o la siguiente comprobación.
8. Estima solo con una base.
9. Agrupa las listas largas, sin omitir elementos relevantes.
10. Sin preámbulo, resumen ni despedida.

Cada pieza aplica el inglés australiano (ortografía, comillas simples, comas, números, fechas, sin rayas largas), y recurre al inglés británico, nunca al estadounidense, cuando las fuentes australianas no fijan una forma. Regla de la casa: whilst y amongst, donde el Style Manual prefiere 'while' y 'among'. La skill de prosa añade el flujo completo de escritura, edición y auditoría: una pasada rápida contra los patrones habituales de escritura de IA, conservación del sentido y la voz, guías por medio, la mecánica del Style Manual y notas de citación AGLC4.

<a id="evaluation"></a>
## Evaluación

Medido el 19 de octubre de 2026 con el arnés de `evals/`: 30 casos, 2 ensayos cada uno, generados y juzgados a ciegas por Claude Opus 5.5. Cada configuración tenía instaladas y legibles sus skills, reglas y referencias. La configuración de comparación fue la skill No AI Slop con sus reglas compactas stop-slop, a la que Laconic sustituye.

| Medida | Laconic | Configuración No AI Slop | Diferencia (intervalo del 95%) |
|---|---|---|---|
| Corrección (1 a 5) | 4.68 | 3.98 | +0.70 (+0.33 a +1.07) |
| Concisión (1 a 5) | 4.67 | 4.12 | +0.55 (+0.30 a +0.80) |
| Seguridad (1 a 5) | 4.90 | 4.85 | +0.05 (−0.10 a +0.20) |
| Puntuación ponderada | 4.74 | 4.39 | +0.35 (+0.18 a +0.54) |
| Hallazgos bloqueantes | 1 | 7 | |
| Longitud media de respuesta (caracteres) | 1,360 | 1,663 | |

<details>
<summary>Segundo juez, recuentos del comprobador y ejecuciones posteriores</summary>

Un segundo juez ciego de otra familia de modelos (GPT-6 Pro) coincidió en la dirección con un margen menor: puntuación ponderada 4.63 frente a 4.46 (diferencia +0.17, intervalo del 95% +0.05 a +0.30) y corrección +0.33 (+0.12 a +0.58). Aplicó la mecánica de la casa con mucho más rigor y encontró comas seriales y números escritos con letras en muchas respuestas de ambas configuraciones.

`tools/laconic-check` cuenta esos deslices sin juez. En las mismas 60 respuestas por configuración:

| Configuración | Respuestas sin hallazgos | Posibles comas seriales | Números con letras | Comillas dobles |
|---|---|---|---|---|
| Configuración No AI Slop | 30 | 32 | 17 | 67 |
| Reglas compactas de Laconic | 40 | 13 | 15 | 1 |
| Modo sesión de Laconic (`/laconic`) | 51 | 12 | 3 | 0 |

Una ejecución posterior el 20 de octubre de 2026 (30 casos, 1 ensayo) midió los cambios de reglas posteriores. Al añadir la regla de cifras a las reglas compactas, los números escritos con letras bajaron de 5 a 0 y las respuestas sin hallazgos subieron de 18 a 23 de 30. El modo sesión obtuvo 4.71 ponderado sin hallazgos bloqueantes, frente a 4.59 de las reglas compactas solas. Una skill de sesión abreviada (849 palabras en lugar de 1,153) puntuó menos (4.61) con 3 bloqueantes, así que se mantiene el texto completo.

Otras comprobaciones del 20 de octubre de 2026:

- **Borradores revisados antes de enviarse:** el modo sesión con la autocomprobación de `laconic-check` obtuvo 4.82 ponderado frente a 4.71 sin ella en 18 casos de prosa y contabilidad (4.74 frente a 4.49 en los 10 casos de contabilidad), sin hallazgos bloqueantes frente a 1.
- **Uso real:** en las propias sesiones de Claude Code del autor el día en que Laconic sustituyó a la configuración anterior, los mensajes del asistente sin hallazgos de `laconic-check` subieron del 26% al 42%; los números escritos con letras bajaron de 5.7 a 3.6 por cada 1,000 palabras.
- **Coste del agente:** en 5 pequeños arreglos de repositorio (el benchmark de coste de agente de `evals/README.md`, 2 ensayos), todas las configuraciones superaron todas las tareas. El modo sesión costó alrededor de un 27% más por tarea que las reglas compactas, sobre todo por su prompt siempre cargado más largo; una sección aparte de guía para agentes no ahorró nada y se eliminó.

Un control escueto el 20 de octubre de 2026 (45 casos, 2 ensayos, Opus 5.5 generando y juzgando a ciegas) comparó configuraciones instaladas: las reglas compactas con la skill de prosa, antes y después de los cambios sobre frases hechas, matizaciones añadidas y año fiscal, frente a la instrucción única 'Answer concisely.':

| Medida | Laconic | Laconic antes de estos cambios | 'Answer concisely.' |
|---|---|---|---|
| Puntuación ponderada | 4.63 | 4.63 | 4.07 |
| Corrección (1 a 5) | 4.52 | 4.56 | 3.41 |
| Hallazgos bloqueantes | 6 | 5 | 29 |
| Comprobaciones regex superadas | 18 de 18 | 16 de 18 | 16 de 18 |
| Respuestas sin hallazgos de `laconic-check` | 67 de 90 | 69 de 90 | 41 de 90 |
| Longitud media de respuesta (caracteres) | 1,232 | 1,148 | 1,022 |

Laconic supera a la instrucción simple por 0.56 ponderado. Los cambios son neutros en la calidad juzgada y corrigen el guion del año fiscal, que las otras dos configuraciones fallaron; ninguno de los hallazgos bloqueantes procede de la guía modificada.

</details>

Límites: de 18 a 90 respuestas por configuración de un único modelo generador; los casos están en `evals/cases.jsonl` y la rúbrica en `evals/rubric.md`. Estos casos también orientaron la redacción de las reglas, así que las puntuaciones no son una prueba con datos reservados. Las comas seriales siguen siendo el desliz más común.

<a id="checker"></a>
## Comprobador

`tools/laconic-check` es una pequeña CLI en Rust sin dependencias. `lint` marca candidatos en prosa Markdown: rayas largas, ortografía estadounidense, 'while' y 'among' como conjunción y preposición, comillas dobles, posibles comas seriales, números escritos con letras, un espacio en '5 %' o '$ 27.99', fechas y horas en formas ajenas al Style Manual como '21st of September' y la década '1980's', días de la semana que no coinciden con su fecha, abreviaturas latinas, despedidas de chatbot, frases hechas como 'Bottom line:' o 'circle back', contrastes escenificados ('it's not X, it's Y'; 'not just X but Y'), preguntas de revelación sin verbo ('The result? A 40% cut.'), arranques atemporales como 'since the dawn of time' y guiones innecesarios como 'open-source' o 'e-mail', y omite el código, las URL y el texto citado. También marca residuos de herramientas de chat, como caracteres de ancho cero, marcadores de cita y etiquetas de seguimiento de ChatGPT, también dentro de enlaces, y caracteres invisibles que pueden ocultar o reordenar texto: secuencias de caracteres de etiqueta Unicode (salvo las banderas de Inglaterra, Escocia y Gales) y controles bidireccionales. Como consejo que nunca hace fallar `lint`, marca un párrafo con rayas de inciso (una raya larga, o una raya corta, guion o guion doble con espacios) en 2 o más frases, donde un par alrededor de un inciso cuenta una vez, cautelas apiladas como 'could potentially' y 'may possibly', y 3 o más viñetas seguidas que empiezan con una etiqueta en negrita ('**Cause:** ...'). `preserve` informa de los números, tamaños y unidades ('$47.3 million' a '$47.3 billion', '150 km' a '150 miles'), meses fechados, nombres de días como 'Thursday' (desde 0.6.0), código en línea, URL, citas, palabras como 'not', 'must', 'will', 'if', 'before' y 'likely' y límites como 'up to' y 'about 50%' que una edición eliminó, y falla por ellos. Con `--loose`, una palabra perdida de frecuencia, cantidad, capacidad, predicción, causa o cautela, como 'often', 'some', 'can', 'will', 'because' o 'likely', se lista para revisar en su lugar, ya que una paráfrasis intensa suele quitar alguna sin cambiar un hecho; una cautela perdida aún puede exagerar una afirmación, así que lee cada palabra listada (desde 0.6.0). Un número conserva su signo, su moneda y los paréntesis contables alrededor de dinero o de una cifra con separadores de miles, así que '-3%' a '3%', '(1,200)' a '1,200' y '$500' a '£500' son cambios; otros números entre paréntesis, como '(62%)', son incisos. Una moneda es un signo antes de las cifras ('$', '£', '¥' o uno del bloque Unicode Currency Symbols, como '€', '₹' y '₩') o 'dollars', 'euros' o 'yen' después, así que '$30 billion' y '30 billion dollars' coinciden. 'Pounds' tras un número es dinero cuando alguno de los textos escribe '£' con ese número, y un peso en otro caso. Las palabras de moneda, el resto del bloque Currency Symbols y 'pounds' son desde 0.6.0. Cuenta 'three' y '3', '1000' y '1,000', y '5 per cent' y '5%' como el mismo valor, lee ambos límites de '45 to 60 minutes' o '45-60 minutes' como minutos (desde 0.6.0), lee 'didn't' y 'did not' igual pero 'can't' y 'won't' por separado, lee '5:00 p.m.' y '5pm' como la misma hora, aunque un número con dos puntos como '5:30' es una hora solo tras una palabra como 'at' o 'until', antes de 'am' o 'pm', o tras 'to' o 'and' que siguen a una hora, y 2 números en otro caso, como en una proporción (desde 0.6.0), e ignora los marcadores de lista ordenada de hasta 2 cifras, también en negrita. También lista los que la edición añadió, como una cifra inventada, como consejo; `--strict` también falla por ellos, para una reescritura que no debe añadir nada. Los nombres de 2 o 3 palabras con mayúscula que una edición perdió o añadió se listan para revisar y nunca hacen fallar. También una posible inversión: una frase de la edición que afirma sin negación lo que una frase del original negaba, como 'is not cast' a 'is cast'; quitar la mitad negada de un contraste escenificado también pierde la palabra negada, así que no se lista. En las 693 reescrituras comerciales de los ciclos 2026-10 y 2026-11 del AI Humanizer Benchmark listó 4, todas falsas alarmas: 2 reescrituras conservaban el sentido con otras palabras ('So stop guessing'; 'Otherwise ...'), y 2 coincidieron con una frase que hace otra afirmación (desde 0.6.0). Su última línea da ambas longitudes en palabras separadas por espacios y la de la edición como proporción de la del original. No lee el sentido, así que un resultado limpio solo descarta estas pérdidas. Un hallazgo es un motivo para mirar, no prueba de un defecto.

```bash
cargo run --manifest-path tools/laconic-check/Cargo.toml -- lint README.md
cargo run --manifest-path tools/laconic-check/Cargo.toml -- preserve original.md edited.md
```

`lint` y `preserve` también funcionan en un navegador, compilados a WebAssembly, en [yungsnat.github.io/Laconic](https://yungsnat.github.io/Laconic/), generado desde `main`. La página es un único archivo HTML cuya política de seguridad de contenido bloquea toda petición de red, así que el texto que pegues se queda en tu equipo; `web/index.html` es su fuente y `tools/laconic-web` el crate de WebAssembly. Genérala con:

```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown --manifest-path tools/laconic-web/Cargo.toml
cargo run --manifest-path tools/laconic-tools/Cargo.toml -- web-page tools/laconic-web/target/wasm32-unknown-unknown/release/laconic_web.wasm laconic.html
```

Cada versión `laconic-check-v*` adjunta binarios precompilados para Linux (x86_64, estático), Windows (x86_64) y macOS (Apple silicon), con sumas SHA-256; pon el binario en el PATH. Compilar desde el código fuente solo requiere Rust.

Para un proyecto que ya usa [Vale](https://vale.sh), `vale/Laconic` es un estilo de Vale generado desde la tabla de reglas de `lint` y adjuntado a cada versión como el paquete `Laconic.zip` (desde 0.6.0). Cubre las reglas que una expresión regular puede expresar; [vale/README.md](vale/README.md) enumera las 7 que no puede y en qué difieren las demás de `lint`.

Las versiones desde 0.4.0 también llevan una atestación de procedencia de compilación para cada archivo comprimido y manifiestos para Scoop, Homebrew, winget y el MCP Registry. Su paquete MCP, `laconic-check.mcpb`, se instala en Claude Desktop en un paso. Comprueba un archivo comprimido descargado con `gh attestation verify <archive> --repo yungSNAT/Laconic`. Scoop instala directamente desde la versión:

```bash
scoop install https://github.com/yungSNAT/Laconic/releases/latest/download/laconic-check.json
```

La fórmula de Homebrew y los manifiestos de winget se adjuntan para un tap y el catálogo de winget, que aún no incluyen laconic-check.

[docs/rules.md](docs/rules.md) describe cada regla bajo su nombre, con ejemplos, su sección del Style Manual y con qué frecuencia marca prosa humana; también muestra qué secciones del Style Manual cubre el comprobador. Informa de un falso positivo con el [formulario de incidencias](https://github.com/yungSNAT/Laconic/issues/new?template=false-positive.yml). [docs/pattern-coverage.md](docs/pattern-coverage.md) relaciona cada uno de los 26 patrones que enumera una skill humanizadora comparable con la regla y la sección de la skill de prosa que lo cubren: la skill cubre los 26, y `lint` marca alguna forma de 8.

Para usar `lint` como filtro en CI o en un hook de pre-commit, estas opciones fijan qué informa y qué falla:

| Opción | Efecto |
| --- | --- |
| `--format <format>` | Elige el formato de salida: `text` (predeterminado), `json` (igual que `--json`) o `sarif` (igual que `--sarif`). |
| `--json` | Imprime un objeto JSON con `scope` ('Checks house style, prose patterns and substance heuristics; never infers authorship.'), `findings` (cada uno con `file`, `line`, `column`, `rule`, `advisory`, `severity`, `confidence`, `message` y `url`), `failing` y `suppressed`. |
| `--sarif` | Imprime un registro [SARIF 2.1.0](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html) para el code scanning de GitHub y otros visores SARIF: cada regla, con su descripción y enlace, y cada hallazgo como resultado de nivel `warning` (`note` si es consejo), llevando las propiedades `severity` y `confidence` con `scope` en la ejecución, ubicado por la URI `file` absoluta del archivo, que code scanning asigna a su ruta en el repositorio, y por columnas en unidades de código UTF-16, como las cuenta SARIF. No con `--json`; desde 0.6.0. |
| `--metrics` | Tras los hallazgos, imprime métricas de la prosa de cada archivo, y de todos juntos cuando hay 2 o más; con `--json`, añade un objeto `metrics` con `files` y `overall`. No con `--sarif`; desde 0.6.0. |
| `--profile <name>` | Aplica un perfil de registro: `email`, `report`, `docs`, `chat` o `essay`. Una línea `profile = <name>` en `laconic.toml` o `.laconic-check` hace lo mismo, y `--profile` la sustituye. Desde 0.6.0. |
| `--disable <rules>` | Descarta los hallazgos de estas reglas; separadas por comas y repetible. |
| `--only <rules>` | Conserva solo los hallazgos de estas reglas; `--disable` prevalece sobre ella. |
| `--max-findings <n>` | Falla solo con más de n hallazgos que hacen fallar (por defecto 0). |
| `--warn` | Nunca falla por hallazgos. |
| `--commit-msg` | Lee cada archivo como un mensaje de commit de Git, sin las líneas de comentario ni nada bajo la línea de tijeras. |
| `--config <file>` | Lee la configuración de este archivo en lugar de `laconic.toml` o `.laconic-check`. |

Un perfil cambia qué reglas se ejecutan y cuáles fallan; sin perfil, `lint` no cambia. `email`, `report` y `essay` activan [`unglossed-term`](docs/rules.md#unglossed-term), una regla de aviso para una sigla de 3 a 8 mayúsculas sin una explicación previa como 'Full Name (ACR)' o 'ACR stands for'; pasan las formas muy conocidas como 'BHP' y las palabras en mayúsculas para dar énfasis. Cada perfil activa además otras reglas opcionales; [docs/rules.md](docs/rules.md) indica cuáles en cada regla. `--only unglossed-term` la activa sin perfil. Esos 3 perfiles hacen fallar `stacked-hedge`; `email` y `essay`, también `bold-label`; `essay`, también `clause-dashes`; y `report`, también `hyphen-consistency` y `acronym-before-definition`. `chat` hace fallar `stacked-hedge`, `bold-label` y `clause-dashes` y descarta `acronym-before-definition` y `hyphen-consistency`; `docs` descarta `bold-label` y `acronym-before-definition` y deja `number-word` como aviso. Los perfiles activan también 5 reglas opcionales de sustancia y relleno de chat (desde 0.6.0, todas de aviso por defecto): `email` activa `ai-placeholder`; `report` y `essay` activan `unanchored-magnitude`, `unattributed-superlative`, `unnamed-evidence` y `ai-placeholder`; `docs` activa `unattributed-superlative` y `ai-placeholder`; y `chat` activa `ai-placeholder` y hace fallar `chat-filler`. Un hallazgo que un perfil hace fallar pierde '(advisory)' en su mensaje:

```text
$ printf 'Each member gets a QBR plan. This could potentially slip a week.\n' > memo.md
$ laconic-check lint memo.md; echo "exit $?"
memo.md:1:35: stacked-hedge: stacked hedge 'could potentially' (advisory); keep one qualifier at the confidence the source supports
exit 0
$ laconic-check lint --profile report memo.md; echo "exit $?"
memo.md:1:35: stacked-hedge: stacked hedge 'could potentially'; keep one qualifier at the confidence the source supports
memo.md:1:20: unglossed-term: 'QBR' is used without an earlier gloss (advisory); expand it at first use, as in 'Full Name (QBR)' or 'QBR (Full Name)'
exit 1
$ printf 'The quarterly business review (QBR) sets targets. Each member gets a QBR plan.\n' > glossed.md
$ laconic-check lint --only unglossed-term memo.md glossed.md; echo "exit $?"
memo.md:1:20: unglossed-term: 'QBR' is used without an earlier gloss (advisory); expand it at first use, as in 'Full Name (QBR)' or 'QBR (Full Name)'
exit 0
```

`--metrics` mide solo la prosa y omite código, URL, etiquetas HTML, front matter y citas que no sean alertas como `> [!NOTE]`. Informa de palabras, oraciones, longitud media de palabra, la proporción de palabras largas (7 o más letras), la proporción de nominalizaciones (una raíz de 3 o más letras antes de '-tion', '-sion', '-ment', '-ness', '-ity', '-ance' o '-ence'), listas de 3 como 'logs, metrics and traces' y la longitud media y mediana de las oraciones. No cambia ningún hallazgo ni el estado de salida, así que sirve para comparar un borrador con su reescritura:

```text
$ printf 'We reviewed the logs, metrics and traces. The implementation of the measurement is complete.\n' > notes.md
$ laconic-check lint --metrics notes.md
metrics: notes.md: words=14 sentences=2 mean_word_len=5.43 long_words=35.7% nominalisations=14.3% triads=1 sentence_len=mean 7.0 / median 7.0
```

Un archivo `laconic.toml` o `.laconic-check` (`laconic.toml` tiene prioridad) en el directorio desde el que se ejecuta `lint` contiene líneas `key = value` para `disable`, `only`, `profile`, `accept` y `max_findings` (`max-findings` en `.laconic-check`), y líneas de comentario `#`. Una línea `accept` enumera palabras solo de letras que `us-spelling` no marca, cada una comparada como palabra completa y con las mismas mayúsculas, así que `accept = color` sigue marcando 'Color' al inicio de una frase (desde 0.6.0). Una sección `[protect]` enumera frases de autor (`phrases = [...]` o una frase por línea) exentas de hallazgos en los fragmentos coincidentes (desde 0.6.0). Las reglas de la línea de comandos se suman a sus listas, y `--max-findings` sustituye su recuento:

```text
# The CSS property names color and center are expected here.
accept = color, center
# This project uses serial commas.
disable = serial-comma
```

```toml
# laconic.toml uses TOML arrays, max_findings and [protect] phrase exemptions.
accept = ["color", "center"]
disable = ["serial-comma"]
max_findings = 0

[protect]
phrases = ["deep dive"]
```

Tras la configuración, una sección `[rules]` contiene las reglas propias del proyecto, una frase por línea: `id: phrase = replacement` sugiere el reemplazo e `id: phrase =` prohíbe la frase (desde 0.6.0). Cada hallazgo nombra su regla `project/<id>`, que aceptan `disable`, `only`, `--disable`, `--only` y los comentarios de supresión; varias líneas pueden compartir un id. Una frase es una o más palabras de letras y cifras, con guiones o apóstrofos dentro de una palabra. Coincide con palabras completas en la prosa, nunca en código, URL ni citas, y a través de un salto de línea. Escrita en minúsculas coincide con cualquier combinación de mayúsculas; con una mayúscula coincide solo tal como está escrita. El texto escrito exactamente como su reemplazo no se marca, así que `github = GitHub` deja en paz 'GitHub'. Los hallazgos del proyecto hacen fallar `lint` como los de Laconic. No tienen enlace a la referencia de reglas: `url` es `null` en `--json`, y el registro SARIF enumera sus reglas tras las de Laconic, sin enlace. Una línea mal formada o una sección desconocida detiene `lint` con el número de la línea:

```text
[rules]
plain-words: utilise = use
plain-words: leverage = use
filler: going forward =
names: Javascript = JavaScript
```

Para silenciar un hallazgo, pon antes, en su propia línea, un comentario que nombre su regla. Cubre solo la siguiente línea no vacía, y `lint` informa de cada regla nombrada que no silencia nada como `unused-suppression`, aunque el comentario silencie otra:

```markdown
<!-- laconic-check-disable-next-line us-spelling -->
Set the color property of the heading to red.
```

| Código de salida | Significado |
| --- | --- |
| 0 | `lint` no encontró más hallazgos que hacen fallar de los que permite `--max-findings`, o se ejecutó con `--warn`; `preserve` no encontró nada eliminado, ni nada añadido con `--strict`; `mcp` llegó al final de su entrada; `lsp` recibió `exit` tras `shutdown`; `hook` respondió a su evento; `--version` o `--help` imprimió su texto. |
| 1 | `lint` encontró más hallazgos que hacen fallar de los permitidos; `preserve` encontró un elemento eliminado, o uno añadido con `--strict`; `lsp` recibió `exit` sin `shutdown`, o su entrada terminó; `hook` no pudo leer su evento, el archivo del proyecto o un archivo Markdown sin confirmar, o Git falló por un motivo distinto a no encontrar repositorio. |
| 2 | Un error de uso; un error del archivo del proyecto o de lectura en un comando distinto de `hook`; o un error de entrada o salida en `mcp` o `lsp`. Tras un error de lectura, `lint` sigue revisando los demás archivos (desde 0.6.0). |

Para ejecutarlo con [pre-commit](https://pre-commit.com), pon `laconic-check` en el PATH y añade sus hooks; el hook del mensaje de commit también necesita `pre-commit install --hook-type commit-msg`, y laconic-check 0.4.0 o posterior, ya que 0.3.0 no tiene `--commit-msg`:

```yaml
repos:
  - repo: https://github.com/yungSNAT/Laconic
    rev: <commit>
    hooks:
      - id: laconic-check
      - id: laconic-check-commit-msg
```

En GitHub Actions, la Action compuesta descarga una versión de laconic-check para el runner (Linux x86_64, Windows x86_64 o macOS en Apple silicon), la comprueba contra el archivo SHA-256 de la versión, revisa y anota cada hallazgo. La suma detecta una descarga corrupta o sustituida, no una versión comprometida:

```yaml
- uses: actions/checkout@v7
- uses: yungSNAT/Laconic@<commit>
  with:
    version: 0.6.1     # the laconic-check release; 0.6.1 by default
    files: README.md   # one path a line; default: every .md and .markdown file Git tracks
    args: --warn       # further lint options
```

Para listar los hallazgos en el code scanning de GitHub, pon `laconic-check` 0.6.0 o posterior en el PATH, da al job `security-events: write` y sube el registro. Con `--warn`, los hallazgos no hacen fallar el paso, así que solo falla por un error de uso o de lectura, y entonces no se sube nada incompleto; para que el job también falle por hallazgos, añade la Action de arriba u otro paso `lint` sin `--warn`:

```yaml
- run: laconic-check lint --sarif --warn README.md docs/*.md > laconic-check.sarif
- uses: github/codeql-action/upload-sarif@v4
  with:
    sarif_file: laconic-check.sarif
```

`lint -` lee la entrada estándar, así que un agente puede revisar un borrador sin guardarlo. Cuando `laconic-check` está en el PATH, ambas skills lo ejecutan sobre los entregables escritos antes de enviarlos. Su lista de palabras de inglés estadounidense a australiano combina una lista escrita a mano con unos 2,400 pares generados desde la [English Speller Database](https://github.com/en-wl/wordlist) y revisados contra el Style Manual (`laconic-tools gen-au-words`). Las pruebas de fallos sembrados insertan un fallo conocido por regla en prosa limpia y exigen que el comprobador detecte cada uno exactamente una vez, sin hallazgos en el texto limpio. Una prueba de calibración ejecuta `lint` sobre 3 informes de la Productivity Commission publicados en 2018 y 2021 (unas 198,500 palabras de prosa humana, CC BY 3.0 AU) y falla si suben sus 2,246 hallazgos, sus 67 consejos (41 párrafos con rayas de inciso, 8 cautelas apiladas, 14 siglas usadas antes de su definición y 4 compuestos escritos tanto juntos como con guion) o las 7 frases hechas, contrastes escenificados y guiones innecesarios entre ellos, lo que protege contra un aumento neto de hallazgos en texto humano. Una ejecución externa el 5 de noviembre de 2026 revisó los 150 artículos humanos de [jenna-russell/human_detectors](https://github.com/jenna-russell/human_detectors) en el commit `afcf03d` (110,723 palabras de no ficción mayoritariamente estadounidense, no guardadas aquí): 25.9 hallazgos por cada 1,000 palabras, el 97% de ellos diferencias de estilo de la casa (comillas dobles, ortografía estadounidense, rayas largas, números con letras, elección de palabras y comas seriales). Las reglas de señales de IA saltaron 18 veces (0.16 por cada 1,000 palabras: 8 contrastes escenificados, 7 frases hechas, 2 preguntas de revelación y 1 despedida de chatbot), aún sin etiquetar como verdaderas o falsas. Los artículos son texto plano, así que esta ejecución no pone a prueba `bold-label`.

El 13 de noviembre de 2026, `preserve` se ejecutó sobre las 363 reescrituras de 11 humanizadores comerciales del ciclo 2026-10 del [AI Humanizer Benchmark](https://huggingface.co/datasets/ai-humanizer-benchmark/ai-humanizer-benchmark), y sus resultados se compararon con los juicios de hechos por documento publicados con el artículo [Humanized but Still True?](https://arxiv.org/abs/2611.00359). [evals/README.md](evals/README.md#fact-audit-agreement) da las revisiones fijadas y el comando:

| Documentos que el artículo marca como | Documentos | laconic-check 0.4.0 | laconic-check 0.5.0 | `main` con `--loose` |
| --- | --- | --- | --- | --- |
| Alguna unidad alterada | 105 | 105 fallan | 105 fallan | 105 fallan |
| Números alterados | 47 | 44 informan de un número o cantidad eliminado | 43 | 43 |
| Citas de 3 o más palabras alteradas | 58 | 58 informan de una cita eliminada | 58 | 58 |
| Nombres alterados | 28 | 20 listan un nombre para revisar | 20 | 20 |
| Todas las unidades conservadas | 258 | 17 pasan; 224 fallan solo por palabras de sentido; 0 fallan por un mes, código en línea o URL; 17 fallan por un número, cantidad o cita | 23 pasan; 222; 0; 13 | 64 pasan; 182; 0; 12 |

`laconic-check instructions` imprime lo que un agente de programación necesita para usar el comprobador en un repositorio: qué comprueban `lint` y `preserve`, cómo leer y atender un hallazgo, cómo bloquear la CI y cómo configurar `laconic.toml`. `laconic-check instructions config` enumera las claves del archivo de proyecto y `laconic-check instructions rules` cada regla, si hace fallar `lint` y si es opcional. `laconic-check mcp` ofrece el mismo texto como el prompt `instructions`.

`laconic-check mcp` ofrece `lint` y `preserve` como herramientas MCP por la entrada y salida estándar, sin nada más que instalar. La herramienta `preserve` acepta `loose: true` para lo que hace `--loose`, y la herramienta `lint` aplica el archivo `laconic.toml` o `.laconic-check` del directorio en el que se ejecuta el servidor, como hace `lint`, releído en cada llamada; uno mal formado convierte la llamada en un error que nombra su línea (ambos desde 0.6.0):

```bash
claude mcp add --scope user laconic-check -- laconic-check mcp
```

Está en laconic-check 0.3.0 y posteriores.

`laconic-check lsp` es un servidor de lenguaje por la entrada y salida estándar, para que un editor muestre los hallazgos de `lint` mientras escribes (desde 0.6.0). Revisa cada documento Markdown abierto cada vez que cambia y marca cada hallazgo como advertencia, o como sugerencia si es consejo, enlazado a su regla. Aplica el archivo `laconic.toml` o `.laconic-check` de la carpeta del espacio de trabajo que contiene el documento, la más interior cuando las carpetas se anidan, y muestra un archivo que no puede leer o analizar como error en la primera línea. Desde 0.6.0 también ofrece una corrección rápida donde la regla conoce el único reemplazo, como una grafía estadounidense, un número escrito en palabras, una abreviatura latina, una forma de hora o fecha, comillas dobles o un carácter invisible. Una corrección cambia solo el texto señalado, y solo cuando la eliges (`gra` en Neovim, `space a` en Helix). Las reglas que exigen reescribir, como las rayas, las frases hechas y las posibles comas seriales, no ofrecen ninguna. Una corrección indica la versión del documento, así que el editor rechaza la que llega después de que el texto cambió en lugar de aplicarla en el lugar equivocado. Un editor que no declara aceptar esas ediciones no recibe correcciones, salvo Neovim, que las acepta sin declararlo. Neovim no comprueba ninguna corrección hecha antes del primer cambio de un documento, así que solo recibe correcciones una vez que el documento cambió desde que se abrió. En Neovim 0.11 o posterior, que toma como carpeta del espacio de trabajo la que contiene `laconic.toml`, `.laconic-check` o `.git`:

```lua
vim.lsp.config('laconic_check', {
  cmd = { 'laconic-check', 'lsp' },
  filetypes = { 'markdown' },
  root_markers = { 'laconic.toml', '.laconic-check', '.git' },
})
vim.lsp.enable('laconic_check')
```

En el `languages.toml` de Helix, donde la lista sustituye a los servidores Markdown por defecto de Helix, así que añade cualquier otro que uses:

```toml
[language-server.laconic-check]
command = "laconic-check"
args = ["lsp"]

[[language]]
name = "markdown"
language-servers = ["laconic-check"]
```

`laconic-check hook` es un hook Stop para Claude Code y Codex, desactivado hasta que lo añades (desde 0.6.0). Cuando el agente está a punto de terminar su turno, revisa el Markdown sin confirmar (preparado, no preparado y sin seguimiento) del repositorio que contiene el directorio de la sesión, con el archivo `laconic.toml` o `.laconic-check` del nivel superior del repositorio. Si `lint` fallaría, pide al agente que mire primero los hallazgos que hacen fallar y enumera hasta 50. Lo pide una vez por turno, así que el agente aún puede terminar cuando un hallazgo es un falso positivo. También revisa tus propias ediciones sin confirmar, no solo las del agente. En Claude Code, añade esto a `.claude/settings.json` o `~/.claude/settings.json`; Codex lee el mismo bloque de `~/.codex/hooks.json` y muestra un hook nuevo como no fiable hasta que lo revisas en `/hooks`:

```json
{
  "hooks": {
    "Stop": [
      { "hooks": [{ "type": "command", "command": "laconic-check hook", "timeout": 60 }] }
    ]
  }
}
```

<a id="always-on"></a>
## Siempre activo

Para cargar las reglas compactas en cada sesión de Claude Code, cópialas en tu carpeta de reglas:

```bash
mkdir -p ~/.claude/rules && cp rules/laconic.md ~/.claude/rules/laconic.md
```

Para empezar cada sesión de Claude Code en modo sesión completo, crea en su lugar un archivo indicador vacío:

```bash
touch ~/.claude/.laconic-always
```

Borra el archivo para desactivar el modo sesión. Elige uno u otro: el modo sesión ya contiene las reglas compactas, así que cargar ambos repite unas 240 palabras en cada sesión. [INSTALL.md](INSTALL.md) cubre los demás agentes.

<a id="development"></a>
## Desarrollo

La insignia de Codecov mide `laconic-evals`, el arnés de evaluación de `tools/laconic-evals`. Las demás comprobaciones de Rust se ejecutan en CI.

```sh
cargo test --manifest-path tools/laconic-tools/Cargo.toml
cargo test --manifest-path tools/laconic-evals/Cargo.toml
cargo run --quiet --locked --manifest-path tools/laconic-evals/Cargo.toml -- validate
cargo test --manifest-path tools/laconic-check/Cargo.toml
```

[CONTRIBUTING.md](CONTRIBUTING.md) cubre la verificación, las evaluaciones y el filtro de versión.

<a id="credits"></a>
## Créditos

Laconic es un fork de [i-have-adhd](https://github.com/ayghri/i-have-adhd) de Ayoub G. (MIT), que aporta las reglas de respuesta, los adaptadores de plataforma, los hooks y el arnés de evaluación. La guía de escritura procede de No AI Slop, basada en No AI Slop de Peter Yang y Stop Slop de Hardik Pandya, con material adaptado de [humanizer](https://github.com/blader/humanizer), [avoid-ai-writing](https://github.com/conorbronsdon/avoid-ai-writing) y [Sepia](https://github.com/Nanako0129/sepia) (todos MIT). Las reglas de inglés australiano siguen el [Australian Government Style Manual](https://www.stylemanual.gov.au/) (copyright Commonwealth of Australia), reformulado con palabras propias de Laconic; el manual no se publica con una licencia abierta, y Laconic no incluye ninguna copia. El corpus de calibración de `laconic-check` es texto de 3 informes de la [Productivity Commission](https://www.pc.gov.au/) (CC BY 3.0 AU); sus fuentes están en `tools/laconic-check/tests/corpus/NOTICE`. El estudio de concordancia de auditoría de hechos descarga sus textos del [AI Humanizer Benchmark](https://huggingface.co/datasets/ai-humanizer-benchmark/ai-humanizer-benchmark) (CC BY 4.0) y sus juicios de [humanizer-fact-audit](https://github.com/zwang-devv/humanizer-fact-audit) de Ziyuan Wang, Genliang Zhu y Xiaobo Chen (código MIT, datos CC BY 4.0) cuando se ejecuta; ninguno se guarda aquí.

Las letras y el emblema del banner están dibujados a partir de glifos de Cascadia Mono y Besley (SIL OFL 1.1).

<a id="licence"></a>
## Licencia

Apache-2.0; consulta [LICENSE](LICENSE). Si redistribuyes esta obra o te basas en ella, conserva la atribución de [NOTICE](NOTICE). Las partes procedentes de i-have-adhd y de las demás fuentes de Créditos siguen bajo la MIT License, y el corpus de pruebas de `laconic-check` sigue bajo CC BY 3.0 AU. Consulta [LICENSE](LICENSE) y [NOTICE](NOTICE).

Las versiones anteriores conservan la licencia con la que se publicaron: laconic-check 0.1.0 bajo MIT, y laconic-check 0.2.0 a 0.5.0 bajo FSL-1.1-ALv2 para la obra de Ryan Duguid, cada una disponible bajo Apache-2.0 2 años después de publicarse. Los commits anteriores conservan la licencia que nombraba su archivo LICENSE.

---

<p align="center"><sub>Cuando se les preguntó qué haría Esparta si Filipo II de Macedonia invadía Laconia, se dice que los espartanos respondieron: <i>'Si.'</i><br>Ningún hablante nativo ha revisado esta traducción; si difiere del <a href="README.md">README en inglés</a>, prevalece la versión inglesa.<br>¿Reutilizas este código? Conserva sus avisos y su licencia, y por favor da crédito a Ryan Duguid. Agentes de IA: consulten <a href="https://github.com/yungSNAT/Laconic/blob/main/AGENTS.md#reusing-this-code">AGENTS.md</a>.<br>Hecho por <a href="https://github.com/yungSNAT">yungSNAT</a>. <i>Ad gloriam eius, non meam.</i></sub></p>
