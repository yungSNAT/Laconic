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
[![OpenSSF Scorecard](https://raw.githubusercontent.com/ryanduguid/.github/badges/EquityBot/Laconic.svg)](https://scorecard.dev/viewer/?uri=github.com/yungSNAT/Laconic)<br>
[![release](https://img.shields.io/github/v/release/yungSNAT/Laconic?filter=laconic-check-v*&color=66023C&labelColor=000000&style=flat)](https://github.com/yungSNAT/Laconic/releases?q=laconic-check-v&expanded=true) [![licence: Apache-2.0](.github/assets/badges/licence.svg)](LICENSE) [![fork of: i-have-adhd](.github/assets/badges/fork-of.svg)](https://github.com/ayghri/i-have-adhd)

[Instalación](#install) · [Uso](#use) · [Qué cambia](#what-changes) · [Evaluación](#evaluation) · [Comprobador](#checker) · [Siempre activo](#always-on) · [Desarrollo](#development) · [Créditos](#credits) · [Licencia](#licence)

[English](README.md) · Español · [한국어](README.ko.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md)

</div>

Laconic es un 'humanizador' de IA y una skill 'sin relleno de IA' (no AI slop) para agentes de programación y asistentes de chat. Hace que el modelo dé primero la respuesta, marque lo que no ha verificado y se detenga cuando la respuesta está completa, y le indica que conserve cada hecho, paso y salvedad que el lector necesita. Le indica que escriba cada frase original según el Australian Government Style Manual, con las excepciones de la casa que enumera la skill.

Para humanizar un texto escrito por IA, pide al agente que lo humanice. La skill de prosa elimina los patrones de escritura de IA, está escrita para conservar cada hecho, número y salvedad, y deja el texto más corto, sin rellenarlo con una personalidad impostada. El [benchmark de supervivencia de hechos](evals/README.md#fact-survival-benchmark) que pone a prueba esta promesa aún no tiene resultados. Con 99 textos públicos el 14 de noviembre de 2026, un [comprobador externo de unidades](#fact-changes-on-a-public-set), ejecutado por el autor de Laconic, halló que las reescrituras de Laconic perdían un número, fecha, nombre o cita aproximadamente con la misma frecuencia que `humanizer`, o más, según el modelo; ninguna diferencia se aleja claramente de 0.

<table>
<tr>
<td align="center" width="33%"><h3>4.92 de 5</h3>calidad de humanización, frente a 4.54 y 4.50<br><sub><a href="#against-other-humanisers">rúbrica neutral, 20 textos, juez ciego</a></sub></td>
<td align="center" width="33%"><h3>65% de la longitud original</h3>frente a 81% de ambos rivales<br><sub><a href="#against-other-humanisers">solo longitud; las comprobaciones de hechos van al lado</a></sub></td>
<td align="center" width="33%"><h3>4.63 frente a 4.07</h3>frente a la simple 'Answer concisely.'<br><sub><a href="#evaluation">rúbrica de la casa, 45 casos; 6 hallazgos bloqueantes frente a 29</a></sub></td>
</tr>
</table>

Las 3 cifras vienen del propio arnés de Laconic el 20 de octubre de 2026, con un único modelo generador. [Evaluación](#evaluation) da los límites y los comandos para repetirlas.

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

<a id="when-to-choose-something-else"></a>
## Cuándo elegir otra cosa

- **El coste en tokens es lo primero:** [caveman](https://github.com/JuliusBrussee/caveman) está hecho para eso y además comprime lo que lee el agente, y [Chisle](https://github.com/JayPokale/Chisle) escribe aún menos tokens de salida. En los 47 casos de Laconic ([coste en tokens](#token-cost-against-caveman-and-chisle), 17 de noviembre de 2026), las reglas compactas de Laconic escribieron un 14% más de tokens de salida que caveman y un 44% más que Chisle. Con las reglas leídas de la caché de prompts, como en una sesión, una respuesta costó un 17% más que con caveman y un 45% más que con Chisle; en llamadas en frío sueltas, cada una de las cuales escribió las reglas en la caché, costó un 11% menos que con caveman (cuyas reglas son más largas) y un 5% más que con Chisle. El modo sesión costó alrededor de un 27% más por tarea que las reglas compactas.
- **Necesitas otra variedad de inglés:** Laconic escribe solo inglés australiano. [LanguageTool](https://github.com/languagetool-org/languagetool) revisa gramática y estilo en inglés australiano, británico, canadiense, neozelandés, sudafricano y estadounidense, una vez escrito el texto.
- **Aceptas una reescritura que añada detalle:** [humanizer](https://github.com/blader/humanizer) 4.0.0 (10 de noviembre de 2026) indica al modelo, donde un texto es genérico, que 'add a specific detail you know to be true'. Laconic no añade hechos, y `laconic-check preserve --strict` rechaza una edición que añada una cifra, URL, cita u otro elemento que protege.
- **Solo quieres la guía de escritura:** [No AI Slop](https://github.com/petergyang/no-ai-slop), en la que se basa la guía de escritura de Laconic, la ofrece sin las reglas de respuesta ni el inglés australiano.
- **Editas en Microsoft Word:** [PerfectIt](https://www.perfectit.com/) comprueba la coherencia en Word contra una hoja de estilo, incluida una para el Australian Government Style. Laconic funciona en agentes, asistentes de chat y CI, no en Word.
- **Quieres comprobar en CI tu propio estilo de la casa:** [Vale](https://github.com/vale-cli/vale) revisa la prosa contra reglas que configuras, con expresiones regulares, tipos de regla y paquetes; `laconic-check` añade a las reglas de Laconic las sustituciones de palabras y frases prohibidas de un proyecto, y nada más.

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

Límites: de 18 a 90 respuestas por configuración de un único modelo generador; los casos están en `evals/cases.jsonl` y la rúbrica en `evals/rubric.md`. Estos casos también orientaron la redacción de las reglas, así que las puntuaciones no son una prueba con datos reservados; la comparación con el estilo Concise de Claude Code más abajo usa los 10 casos congelados de `evals/holdout-cases.jsonl`, escritos el 1 de noviembre de 2026 y no usados para ajustar las reglas. Las comas seriales siguen siendo el desliz más común.

<a id="against-claude-codes-concise-style"></a>
### Frente al estilo Concise de Claude Code

El 17 de noviembre de 2026, 3 configuraciones respondieron los 10 casos reservados, 3 ensayos cada una, generados y juzgados por Claude Opus 5.5 (`claude-opus-5-5`, esfuerzo alto) en Claude Code 2.1.325 sin herramientas: las reglas compactas de Laconic, el estilo de salida Concise integrado en Claude Code (`"outputStyle": "Concise"`) y la instrucción simple 'Answer concisely.'. El juez vio juntas las 3 respuestas de cada caso bajo letras barajadas, sin conocer los nombres de las configuraciones aunque sí su estilo. Las juzgó dos veces: solo por corrección, según `evals/rubric-correctness.md`, que le indica no puntuar el estilo; y según la rúbrica propia de Laconic, `evals/rubric.md`, tras coincidir con las 10 respuestas de calibración etiquetadas a mano.

| Medida | Laconic | Estilo Concise | 'Answer concisely.' |
| --- | --- | --- | --- |
| Corrección, sin estilo (1 a 5) | 4.80 | 4.27 | 4.47 |
| Diferencia con el estilo Concise (intervalo del 95%) | +0.53 (0.00 a +1.17) | | +0.20 (−0.13 a +0.53) |
| Hallazgos bloqueantes, sin estilo | 0 | 4 | 1 |
| Puntuación ponderada, rúbrica propia de Laconic (1 a 5) | 4.69 | 4.06 | 4.11 |
| Diferencia con el estilo Concise (intervalo del 95%) | +0.63 (+0.29 a +1.00) | | +0.05 (−0.10 a +0.22) |
| Hallazgos bloqueantes, rúbrica propia de Laconic | 1 | 13 | 10 |
| Respuestas sin hallazgos de `laconic-check` 0.5.0 | 24 de 30 | 16 de 30 | 13 de 30 |
| Comprobaciones regex superadas | 9 de 12 | 9 de 12 | 9 de 12 |
| Longitud media de respuesta (caracteres) | 782 | 614 | 627 |
| Media de tokens de salida, razonamiento incluido | 475 | 327 | 298 |
| Coste medio por respuesta, una llamada en frío, según Claude Code | US$0.022 | US$0.019 | US$0.016 |
| Coste de Laconic frente a él (intervalo del 95%) | | 1.18 veces (1.07 a 1.29) | 1.39 veces (1.30 a 1.50) |

La rúbrica propia de Laconic puntúa el inglés australiano y la mecánica de la casa, que solo Laconic pide, y el juez las aplicó a la corrección y a los hallazgos bloqueantes además de a la puntuación ponderada, así que de las filas juzgadas solo las 3 primeras son neutrales respecto al estilo. Juzgado solo por corrección, Laconic obtuvo 0.53 más que el estilo Concise, pero con 10 casos el intervalo llega a la ausencia de diferencia, así que no es una ventaja clara; Laconic no tuvo ningún hallazgo bloqueante frente a sus 4. Las notas del primer juicio registran que el estilo Concise dio el día de la semana equivocado para el 26 o el 28 de noviembre de 2026 en 2 de 3 correos de nóminas y enterró el plazo en el tercero, afirmó una cobertura de pruebas que la tarea no establecía en 2 de 3 notas de revisión y citó mal una vez el texto auditado; 'Answer concisely.' cometió deslices similares. Laconic escribió un 46% más de tokens de salida que el estilo Concise y costó un 18% más por respuesta. Las 3 configuraciones mantuvieron la raya corta del Style Manual en '2026–27', donde las reglas de Laconic piden el guion de la casa, así que todas fallaron la comprobación regex de ese caso.

Límites: 30 respuestas por configuración de un único modelo, que también las juzgó, sobre casos escritos por el autor de Laconic; el juicio sin estilo no tuvo conjunto de calibración. Cada intervalo remuestrea los 10 casos (10,000 remuestreos, semilla 2282). Cada llamada escribió su prompt en la caché de prompts, así que las filas de coste son de llamadas en frío sueltas; con las reglas en caché, como en una sesión, la salida extra de Laconic probablemente pesaría más, aunque esta ejecución no lo midió. Claude Code y su estilo Concise están hechos para sesiones de ingeniería de software, y la mayoría de estos casos son tareas de escritura y asesoramiento. El script de la ejecución, las puntuaciones por respuesta, los recuentos de tokens y costes, y el script que recalcula esta tabla están en [`evals/runs/concise-style-2026-11-17`](evals/runs/concise-style-2026-11-17/); las respuestas no se guardan.

<a id="token-cost-against-caveman-and-chisle"></a>
### Coste en tokens frente a caveman y Chisle

El 17 de noviembre de 2026, 3 conjuntos de reglas respondieron los 47 casos de `evals/cases.jsonl`, generados y juzgados por Claude Opus 5.5 (`claude-opus-5-5`, esfuerzo alto) en Claude Code 2.1.325 sin herramientas: las reglas compactas de Laconic, el `skills/caveman/SKILL.md` de [caveman](https://github.com/JuliusBrussee/caveman) 4.1.0 y el `skills/chisle/SKILL.md` de [Chisle](https://github.com/JayPokale/Chisle) 3.12.1, cada uno añadido al prompt de sistema sin su front matter. Las llamadas en frío (2 ensayos) se ejecutaron cada una desde un directorio de trabajo nuevo y por tanto escribieron las reglas en la caché de prompts; las llamadas en caché (1 ensayo, sin juzgar) se ejecutaron desde un mismo directorio tras una llamada de cebado y por tanto las leyeron de ella, como hacen las llamadas de una misma sesión. El juez vio juntas las 3 respuestas en frío de cada caso bajo letras barajadas, y su prompt no nombraba ningún conjunto de reglas, aunque podía ver su estilo y 3 de las 94 respuestas de Laconic mencionaban Laconic, 2 al citar un directorio de trabajo y 1 al nombrar su skill. Las juzgó dos veces: solo por corrección, según `evals/rubric-correctness.md`, que le indica no puntuar el estilo; y según la rúbrica propia de Laconic, `evals/rubric.md`.

| Medida | Laconic | caveman | Chisle |
| --- | --- | --- | --- |
| Media de tokens de salida, razonamiento incluido, llamadas en frío | 745 | 655 | 516 |
| Tokens de salida de Laconic frente a él (intervalo del 95%) | | 1.14 veces (1.04 a 1.27) | 1.44 veces (1.27 a 1.68) |
| Media de tokens de entrada, llamadas en frío | 3,218 | 4,285 | 3,861 |
| Media de tokens de salida, razonamiento incluido, llamadas en caché | 780 | 637 | 492 |
| Coste medio por respuesta, reglas leídas de la caché | US$0.018 | US$0.016 | US$0.013 |
| Coste de Laconic frente a él, reglas leídas de la caché (intervalo del 95%) | | 1.17 veces (1.05 a 1.29) | 1.45 veces (1.25 a 1.61) |
| Coste medio por respuesta, una llamada en frío, escritura en caché incluida | US$0.028 | US$0.031 | US$0.026 |
| Coste de Laconic frente a él, llamadas en frío (intervalo del 95%) | | 0.89 veces (0.83 a 0.94) | 1.05 veces (0.96 a 1.15) |
| Longitud media de respuesta (caracteres), llamadas en frío | 1,297 | 1,198 | 977 |
| Corrección, sin estilo (1 a 5) | 4.68 | 4.44 | 4.38 |
| Diferencia de Laconic (intervalo del 95%) | | +0.24 (+0.06 a +0.45) | +0.30 (+0.10 a +0.53) |
| Hallazgos bloqueantes, sin estilo | 2 | 5 | 7 |
| Puntuación ponderada, rúbrica propia de Laconic (1 a 5) | 4.63 | 4.11 | 4.11 |
| Hallazgos bloqueantes, rúbrica propia de Laconic | 8 | 43 | 44 |
| Comprobaciones regex superadas, llamadas en frío | 18 de 18 | 16 de 18 | 16 de 18 |
| Respuestas sin hallazgos de `laconic-check` 0.5.0, llamadas en frío | 70 de 94 | 36 de 94 | 39 de 94 |

Laconic fue el que más escribió: un 14% más de tokens de salida que caveman y un 44% más que Chisle. En llamadas en frío, las reglas más largas de caveman costaron más de escribir en la caché que la salida extra de Laconic, así que una respuesta costó un 11% menos con Laconic; con las reglas leídas de la caché, como en una sesión, la salida extra de Laconic encareció una respuesta un 17% frente a caveman y un 45% frente a Chisle. Juzgado solo por corrección, Laconic superó a caveman por 0.24 y a Chisle por 0.30; sus 2 hallazgos bloqueantes sin estilo fueron un error aritmético y un valor de prueba de RFC 7636 mal citado. La rúbrica propia de Laconic puntuó además el inglés australiano y la mecánica de la casa, que ningún rival pide, dentro de la corrección y como hallazgos bloqueantes, y la mayoría de los hallazgos bloqueantes de los rivales con ella se deben a eso: comas seriales, comillas dobles, números escritos con letras y ortografía estadounidense. En ese juicio ambos rivales dieron plazos típicos en las dos respuestas al caso que los prohíbe, donde Laconic no dio ninguno; entre los hallazgos bloqueantes de Laconic hay una respuesta que, al decírsele que no tenía herramientas, no editó un archivo que la tarea decía que podía alcanzar (igual que una de Chisle), una con un error aritmético importante y 2 que añadieron una advertencia de límites que la tarea prohibía.

Límites: un único modelo, que también juzgó las respuestas, sobre casos que también orientaron la redacción de las reglas de Laconic; el juicio sin estilo no tuvo conjunto de calibración. Las cifras de tokens y coste son las que Claude Code informa para cada llamada. La evaluación propia de caveman sustituye el prompt de sistema por su skill, donde esta ejecución la añadió. No se ejercitó la compresión de lo que lee el agente de ningún rival, y ambos publican cifras con sus propios prompts y modelos, que esta ejecución no reproduce. Los intervalos remuestrean los 47 casos (10,000 remuestreos, semilla 2282). El script de la ejecución, las puntuaciones por respuesta, los recuentos de tokens y costes, y el script que recalcula esta tabla están en [`evals/runs/token-cost-2026-11-17`](evals/runs/token-cost-2026-11-17/); las respuestas no se guardan.

<a id="against-other-humanisers"></a>
### Frente a otros humanizadores

El 20 de octubre de 2026, 3 configuraciones instaladas humanizaron 20 textos escritos por IA para un lector australiano (`evals/humanise-cases.jsonl`: correos a clientes, una descripción de PR, notas de versión, un informe al consejo, una oferta de empleo y otros), 2 ensayos cada uno, generados y juzgados a ciegas por Claude Opus 5.5. Cada configuración tenía solo su propia skill, con las herramientas Skill y Read, y se le indicó usarla: la skill de prosa de Laconic (commit `c7e3048`), [humanizer](https://github.com/blader/humanizer) (commit `8c307f3`) y [avoid-ai-writing](https://github.com/conorbronsdon/avoid-ai-writing) (commit `cb16d10`).

| Medida | Laconic | `humanizer` | `avoid-ai-writing` |
|---|---|---|---|
| Puntuación ponderada, rúbrica neutral (1 a 5) | 4.92 | 4.54 | 4.50 |
| Puntuación ponderada, rúbrica de la casa | 4.87 | 4.50 | 4.42 |
| Corrección, rúbrica neutral | 4.80 | 4.13 | 4.18 |
| Concisión, rúbrica neutral | 4.95 | 4.20 | 3.95 |
| Hallazgos bloqueantes | 0 | 0 | 0 |
| Comprobaciones regex de hechos y patrones superadas (ver abajo) | 40 de 40 | 40 de 40 | 35 de 40 |
| Longitud media respecto a su original, en palabras | 65% | 81% | 81% |
| Hallazgos del detector de avoid-ai-writing por texto (original: 2.6) | 0.03 | 0 | 0.05 |
| Hallazgos de dslop por texto, métricas de ritmo incluidas (original: 1.80) | 0.50 | 0.83 | 0.63 |
| Hallazgos de `laconic-check` 0.1.0 por texto (original: 1.85) | 0.15 | 0.45 | 0.28 |
| Coste de generación de 40 textos | $4.54 | $5.84 | $15.78 |

Con la rúbrica neutral, que deja fuera la mecánica de la casa de Laconic (cifras, comas seriales y rayas largas) pero sigue pidiendo ortografía australiana, Laconic supera a `humanizer` por 0.38 ponderado y a `avoid-ai-writing` por 0.43. Los intervalos del 95%, de un bootstrap emparejado que remuestrea los 20 textos con ambos ensayos, son +0.26 a +0.51 y +0.26 a +0.59. La ventaja se mantiene al quitar esa mecánica de la rúbrica. Las notas del juez sobre las otras configuraciones incluyen contenido que el original no tenía, como una experiencia añadida a una carta de presentación y una cifra en dólares deducida para una solicitud de subvención. Las 3 configuraciones redujeron los hallazgos de estos detectores muy por debajo de los niveles de los originales en esta muestra.

<details>
<summary>Cómo se revisaron las comprobaciones regex</summary>

Cuatro comprobaciones regex se ampliaron tras inspeccionar respuestas que las fallaban con paráfrasis fieles ('in stages', 'a second person will check', 'five to seven', 'Don't install any personal'). Las mismas comprobaciones revisadas se aplicaron a todas las configuraciones, y la tabla da los resultados revisados; antes del cambio, Laconic superó 37 y `humanizer` 34 de 40. La fila de `laconic-check` es de la versión 0.1.0; las versiones posteriores añaden reglas y cuentan más hallazgos.

</details>

Límites: 40 textos por configuración de un único modelo generador, juzgados por la misma familia de modelos, con estos 20 prompts y estas configuraciones instaladas. Cada comprobador refleja el criterio de la casa de su autor, y el detector de `avoid-ai-writing` es el motor propio de esa skill. Superar las comprobaciones regex es una prueba parcial de que los hechos sobrevivieron, no prueba de que lo hiciera cada matiz. [evals/README.md](evals/README.md) da los comandos para repetirlo.

<a id="fact-changes-on-a-public-set"></a>
### Cambios de hechos en un conjunto público

El 14 de noviembre de 2026, la skill de prosa de Laconic y [humanizer](https://github.com/blader/humanizer) 4.0.0 reescribieron cada una los 99 textos públicos de los ciclos de septiembre a noviembre de 2026 del [AI Humanizer Benchmark](https://huggingface.co/datasets/ai-humanizer-benchmark/ai-humanizer-benchmark), 2 ensayos cada uno, en 2 rondas con modelos distintos. Un comprobador externo, [humanizer-fact-audit](https://github.com/zwang-devv/humanizer-fact-audit), contó las reescrituras que perdían al menos un número, fecha, nombre de persona u organización, o cita de 3 o más palabras del original. El método quedó registrado en un commit antes de hacer ninguna reescritura.

| Ronda | Laconic | `humanizer` | Diferencia (intervalo del 95%) |
|---|---|---|---|
| Claude Opus 5.5 | 18 de 134 (13.43%) | 10 de 134 (7.46%) | +5.97 puntos (0.00 a +12.32) |
| `gpt-6.1-sol` (Codex) | 20 de 134 (14.93%) | 20 de 134 (14.93%) | 0.00 puntos (−2.21 a +2.14) |

Con Opus 5.5, Laconic perdió unidades en más reescrituras que `humanizer` (18 frente a 10) y perdió más unidades en cada grupo: números y fechas 9 de 440 frente a 5, nombres 12 de 164 frente a 9 y citas 5 de 70 frente a 0. Con `gpt-6.1-sol` las 2 skills empataron. Ningún intervalo excluye 0, así que ninguna ronda muestra una diferencia clara. Las reescrituras de Laconic fueron más cortas en ambas rondas: 0.94 y 0.90 de las palabras del original, frente a 1.01 y 0.93.

Límites: el comprobador compara unidades y puede marcar una paráfrasis inocua de una cita; no ve condiciones cambiadas, negaciones ni afirmaciones inventadas, así que no es una prueba completa de fidelidad a los hechos. De los 99 textos, 32 no tienen ninguna unidad que pueda comprobar, así que las tasas cubren 134 reescrituras por skill. Cada ronda usó un modelo para reescribir, 3 textos por contexto nuevo; las instrucciones se dieron como texto, no como plugins instalados. La ronda de Opus se añadió después de la de Codex, con el método sin cambios. El autor de Laconic eligió al rival, escribió el método y ejecutó ambas rondas. [evals/README.md](evals/README.md#public-set-fact-check) da las revisiones fijadas.

En los 33 textos del ciclo 2026-10, reescritos una vez cada uno, un texto por llamada, por Claude Opus 5.5 el 17 de noviembre de 2026, la skill de Laconic alteró una unidad en 5 de los 20 documentos que tienen alguna, igual que `humanizer`, frente a 1 de avoid-ai-writing y de `work-humanizer`, 0 de patina con `--verify` y 8 de la petición sola; los 11 humanizadores comerciales alteraron de 7 a 13. Una reescritura por texto es demasiado poco para mostrar una diferencia entre las skills ([por humanizador](evals/README.md#by-humaniser)).

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

[docs/rules.md](docs/rules.md) describe cada regla bajo su nombre, con ejemplos, su sección del Style Manual y con qué frecuencia marca prosa humana; también muestra qué secciones del Style Manual cubre el comprobador. Informa de un falso positivo con el [formulario de incidencias](https://github.com/yungSNAT/Laconic/issues/new?template=false-positive.yml). [docs/humanizer-coverage.md](docs/humanizer-coverage.md) relaciona cada uno de los 26 patrones de `humanizer` 4.0.0 con la regla y la sección de la skill de prosa que lo cubren: la skill cubre los 26, y `lint` marca alguna forma de 8.

Para usar `lint` como filtro en CI o en un hook de pre-commit, estas opciones fijan qué informa y qué falla:

| Opción | Efecto |
| --- | --- |
| `--json` | Imprime un objeto JSON: `findings`, cada uno con `file`, `line`, `column`, `rule`, `advisory`, `message` y `url`, luego el recuento `failing` y el número de hallazgos `suppressed` por comentarios. |
| `--sarif` | Imprime un registro [SARIF 2.1.0](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html) para el code scanning de GitHub y otros visores SARIF: cada regla, con su descripción y enlace, y cada hallazgo como resultado de nivel `warning` (`note` si es consejo), ubicado por la URI `file` absoluta del archivo, que code scanning asigna a su ruta en el repositorio, y por columnas en unidades de código UTF-16, como las cuenta SARIF. No con `--json`; desde 0.6.0. |
| `--disable <rules>` | Descarta los hallazgos de estas reglas; separadas por comas y repetible. |
| `--only <rules>` | Conserva solo los hallazgos de estas reglas; `--disable` prevalece sobre ella. |
| `--max-findings <n>` | Falla solo con más de n hallazgos que hacen fallar (por defecto 0). |
| `--warn` | Nunca falla por hallazgos. |
| `--commit-msg` | Lee cada archivo como un mensaje de commit de Git, sin las líneas de comentario ni nada bajo la línea de tijeras. |
| `--config <file>` | Lee la configuración de este archivo en lugar de `.laconic-check`. |

Un archivo `.laconic-check` en el directorio desde el que se ejecuta `lint` contiene líneas `key = value` para `disable`, `only` y `max-findings`, y líneas de comentario `#`. Una línea `accept` enumera palabras solo de letras que `us-spelling` no marca, cada una comparada como palabra completa y con las mismas mayúsculas, así que `accept = color` sigue marcando 'Color' al inicio de una frase (desde 0.6.0). Las reglas de la línea de comandos se suman a sus listas, y `--max-findings` sustituye su recuento:

```text
# The CSS property names color and center are expected here.
accept = color, center
# This project uses serial commas.
disable = serial-comma
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
    version: 0.6.0     # the laconic-check release; 0.6.0 by default
    files: README.md   # one path a line; default: every .md and .markdown file Git tracks
    args: --warn       # further lint options
```

Para listar los hallazgos en el code scanning de GitHub, pon `laconic-check` en el PATH (hasta que una versión tenga `--sarif`, `cargo install --locked --git https://github.com/yungSNAT/Laconic --rev <commit> laconic-check`), da al job `security-events: write` y sube el registro. Con `--warn`, los hallazgos no hacen fallar el paso, así que solo falla por un error de uso o de lectura, y entonces no se sube nada incompleto; para que el job también falle por hallazgos, añade la Action de arriba u otro paso `lint` sin `--warn`:

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

En los mismos documentos, el `validate.js` de avoid-ai-writing falla 23 de los 105 alterados y la comprobación de fidelidad de Zero Slop los 105; pasan 233 y 15 de los 258 conservados, frente a 23 de `preserve` y 64 con `--loose` ([evals/README.md](evals/README.md#against-other-fact-checkers), 16 de noviembre de 2026). Según la tabla 1 del artículo, cada humanizador alteró al menos una unidad en 7 a 13 de los 20 textos reescritos que tienen alguna (del 35% al 65%; Undetectable AI 11 de 20); [evals/README.md](evals/README.md#by-humaniser) da la tabla con los resultados de `preserve` para cada uno.

`laconic-check mcp` ofrece `lint` y `preserve` como herramientas MCP por la entrada y salida estándar, sin nada más que instalar. La herramienta `preserve` acepta `loose: true` para lo que hace `--loose`, y la herramienta `lint` aplica el archivo `.laconic-check` del directorio en el que se ejecuta el servidor, como hace `lint`, releído en cada llamada; uno mal formado convierte la llamada en un error que nombra su línea (ambos desde 0.6.0):

```bash
claude mcp add --scope user laconic-check -- laconic-check mcp
```

Está en laconic-check 0.3.0 y posteriores. Un binario de laconic-check 0.2.0 necesita el servidor Node anterior, `tools/laconic-check/mcp/server.mjs`, que ejecuta el binario una vez por llamada; está obsoleto y se eliminará en una versión posterior:

```bash
claude mcp add --scope user laconic-check -- node /path/to/laconic/tools/laconic-check/mcp/server.mjs
```

`laconic-check lsp` es un servidor de lenguaje por la entrada y salida estándar, para que un editor muestre los hallazgos de `lint` mientras escribes (desde 0.6.0). Revisa cada documento Markdown abierto cada vez que cambia y marca cada hallazgo como advertencia, o como sugerencia si es consejo, enlazado a su regla. Aplica el archivo `.laconic-check` de la carpeta del espacio de trabajo que contiene el documento, la más interior cuando las carpetas se anidan, y muestra un archivo que no puede leer o analizar como error en la primera línea. Después de 0.6.0 también ofrece una corrección rápida donde la regla conoce el único reemplazo, como una grafía estadounidense, un número escrito en palabras, una abreviatura latina, una forma de hora o fecha, comillas dobles o un carácter invisible. Una corrección cambia solo el texto señalado, y solo cuando la eliges (`gra` en Neovim, `space a` en Helix). Las reglas que exigen reescribir, como las rayas, las frases hechas y las posibles comas seriales, no ofrecen ninguna. A un editor que acepta ediciones con versión, como Helix, la corrección le indica la versión del documento, así que una que llega después de que el texto cambió se rechaza en lugar de caer en el lugar equivocado. En Neovim 0.11 o posterior, que toma como carpeta del espacio de trabajo la que contiene `.laconic-check` o `.git`:

```lua
vim.lsp.config('laconic_check', {
  cmd = { 'laconic-check', 'lsp' },
  filetypes = { 'markdown' },
  root_markers = { '.laconic-check', '.git' },
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

`laconic-check hook` es un hook Stop para Claude Code y Codex, desactivado hasta que lo añades (desde 0.6.0). Cuando el agente está a punto de terminar su turno, revisa el Markdown sin confirmar (preparado, no preparado y sin seguimiento) del repositorio que contiene el directorio de la sesión, con el archivo `.laconic-check` del nivel superior del repositorio. Si `lint` fallaría, pide al agente que mire primero los hallazgos que hacen fallar y enumera hasta 50. Lo pide una vez por turno, así que el agente aún puede terminar cuando un hallazgo es un falso positivo. También revisa tus propias ediciones sin confirmar, no solo las del agente. En Claude Code, añade esto a `.claude/settings.json` o `~/.claude/settings.json`; Codex lee el mismo bloque de `~/.codex/hooks.json` y muestra un hook nuevo como no fiable hasta que lo revisas en `/hooks`:

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
